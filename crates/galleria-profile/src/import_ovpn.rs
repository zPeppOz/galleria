use crate::*;
use galleria_ovpn::*;
pub fn import_openvpn(
    text: &str,
    name: &str,
    read_file: &dyn Fn(&str) -> std::io::Result<String>,
) -> Result<Imported, ImportError> {
    import_openvpn_bytes(text, name, &|path| read_file(path).map(String::into_bytes))
}
/// Byte-oriented import supports binary PKCS#12 references; the text API remains an adapter.
pub fn import_openvpn_bytes(
    text: &str,
    name: &str,
    read_file: &dyn Fn(&str) -> std::io::Result<Vec<u8>>,
) -> Result<Imported, ImportError> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use zeroize::{Zeroize, Zeroizing};
    let parsed = parse(text).map_err(|_| ImportError::Invalid)?;
    let mut items = vec![];
    let mut secrets = vec![];
    for item in parsed {
        match item {
            Item::Directive { line, name, args } if name == "dev" => {
                if args.first().is_some_and(|s| s.starts_with("tap")) {
                    items.push(Item::Directive {
                        line,
                        name: "dev-type".into(),
                        args: vec!["tap".into()],
                    });
                }
            }
            Item::Directive { line, name, args } if classify(&name, &args) == Class::FileRef => {
                let path = args.first().ok_or(ImportError::Syntax(line))?;
                let mut raw = Zeroizing::new(
                    read_file(path).map_err(|_| ImportError::MissingFile(path.clone()))?,
                );
                let body = Zeroizing::new(if name == "pkcs12" {
                    let encoded = Zeroizing::new(STANDARD.encode(raw.as_slice()));
                    let mut wrapped = String::new();
                    for (index, ch) in encoded.chars().enumerate() {
                        if index > 0 && index % 64 == 0 {
                            wrapped.push('\n');
                        }
                        wrapped.push(ch);
                    }
                    wrapped
                } else {
                    String::from_utf8(std::mem::take(&mut *raw)).map_err(|e| {
                        let mut bytes = e.into_bytes();
                        bytes.zeroize();
                        ImportError::Invalid
                    })?
                });
                let block = Zeroizing::new(format!(
                    "<{name}>\n{}\n</{name}>\n",
                    body.trim_end_matches('\n')
                ));
                let mut parsed = parse(&block).map_err(|_| ImportError::Invalid)?;
                if parsed.len() != 1 || !matches!(&parsed[0],Item::Block {tag,..} if tag==&name) {
                    return Err(ImportError::Invalid);
                }
                items.append(&mut parsed);
                if name == "tls-auth"
                    && let Some(direction) = args.get(1)
                {
                    items.push(Item::Directive {
                        line,
                        name: "key-direction".into(),
                        args: vec![direction.clone()],
                    });
                }
            }
            other => items.push(other),
        }
    }
    let filtered = import_filter(items);
    let mut warnings: Vec<_> = filtered
        .warnings
        .into_iter()
        .map(|w| ImportWarning {
            line: w.line,
            directive: w.name,
            kind: if w.class == Class::Dangerous {
                WarningKind::Dangerous
            } else {
                WarningKind::Unsupported
            },
        })
        .collect();
    let tls_client =
        has_directive(&filtered.items, "client") || has_directive(&filtered.items, "tls-client");
    let mut public = vec![];
    for item in filtered.items {
        match item {
            Item::Block { tag, body, .. } if SECRET_BLOCKS.contains(&tag.as_str()) => {
                secrets.push((
                    format!("openvpn.block.{tag}"),
                    galleria_ipc::Secret::new(body),
                ))
            }
            other => public.push(other),
        }
    }
    if !tls_client {
        warnings.push(ImportWarning {
            line: 0,
            directive: "tls-client".into(),
            kind: WarningKind::NoTlsClient,
        });
    }
    let config = render(&public);
    check_strict(&parse(&config).map_err(|_| ImportError::Invalid)?)
        .map_err(|_| ImportError::Invalid)?;
    let mut profile = Profile::new(name, ProtocolConfig::Openvpn(OpenVpnProfile { config }));
    profile.secret_refs = secrets.iter().map(|(k, _)| k.clone()).collect();
    Ok(Imported {
        profile,
        secrets,
        warnings,
    })
}
