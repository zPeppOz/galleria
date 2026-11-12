#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    It,
}
impl Lang {
    pub fn from_env() -> Self {
        let lang = std::env::var("LC_ALL")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("LANG").ok())
            .unwrap_or_default();
        if lang.starts_with("it") {
            Self::It
        } else {
            Self::En
        }
    }
}
pub fn text(code: &str, lang: Lang) -> &'static str {
    let (en, it) = match code {
        "hello_required" => (
            "Daemon handshake required",
            "Serve la negoziazione con il demone",
        ),
        "incompatible_protocol" => (
            "Incompatible protocol: update Galleria and restart the daemon",
            "Protocollo incompatibile: aggiorna Galleria e riavvia il demone",
        ),
        "message_too_large" => (
            "Message exceeds the protocol limit",
            "Il messaggio supera il limite del protocollo",
        ),
        "too_many_clients" => (
            "Too many clients; try again later",
            "Troppi client; riprova più tardi",
        ),
        "already_active" => (
            "A VPN is already active; use --replace to replace it",
            "Una VPN è già attiva; usa --replace per sostituirla",
        ),
        "not_active" => ("No active VPN session", "Nessuna sessione VPN attiva"),
        "invalid_config" => ("Invalid VPN configuration", "Configurazione VPN non valida"),
        "profile_rejected" => (
            "Profile contains unsupported directives",
            "Il profilo contiene direttive non supportate",
        ),
        "backend_unavailable" => (
            "VPN backend unavailable; check the installed OpenVPN version or WireGuard module",
            "Backend VPN non disponibile; controlla la versione OpenVPN o il modulo WireGuard",
        ),
        "input_not_expected" => (
            "Input request expired or no longer expected",
            "La richiesta del dato è scaduta o non è più attesa",
        ),
        "auth_failed" => ("Authentication failed", "Autenticazione fallita"),
        "input_timeout" => ("Input timed out", "Tempo per inserire il dato scaduto"),
        "server_unreachable" => (
            "Server unreachable after all attempts",
            "Server irraggiungibile dopo tutti i tentativi",
        ),
        "tunnel_setup_failed" => (
            "Cannot configure the tunnel",
            "Impossibile configurare il tunnel",
        ),
        "net_setup_failed" => (
            "Cannot apply the VPN network configuration",
            "Impossibile applicare la rete VPN",
        ),
        "dns_unavailable" => (
            "systemd-resolved is unavailable",
            "systemd-resolved non è disponibile",
        ),
        "unsupported_challenge" => (
            "This authentication challenge is unsupported",
            "Questo tipo di autenticazione non è supportato",
        ),
        "backend_exited" => (
            "VPN backend exited unexpectedly",
            "Il backend VPN si è chiuso inaspettatamente",
        ),
        "permission_denied" => (
            "Permission denied: join group galleria with sudo usermod -aG galleria USER, then log in again",
            "Permesso negato: entra nel gruppo galleria con sudo usermod -aG galleria UTENTE, poi accedi di nuovo",
        ),
        "daemon_unreachable" => (
            "Cannot reach galleriad; check that the service is running",
            "Impossibile raggiungere galleriad; controlla che il servizio sia avviato",
        ),
        "needs_input" => (
            "Input requires a terminal; provide --username and use --password-stdin or --otp-stdin for scripts",
            "Serve un terminale; per gli script usa --username e --password-stdin o --otp-stdin",
        ),
        _ => ("Internal Galleria error", "Errore interno di Galleria"),
    };
    if lang == Lang::It { it } else { en }
}
pub fn incompatible_message(data: &serde_json::Value, lang: Lang) -> String {
    format!(
        "{}; daemon {} (protocol {}..{}), client {} (protocol {})",
        text("incompatible_protocol", lang),
        data["daemon_version"].as_str().unwrap_or("?"),
        data["daemon_min"],
        data["daemon_max"],
        env!("CARGO_PKG_VERSION"),
        galleria_ipc::PROTOCOL_VERSION
    )
}
