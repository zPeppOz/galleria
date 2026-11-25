import { render } from "@testing-library/react";
import { useState, type ComponentType } from "react";
import { I18n } from "../i18n";
import { toDraft, type Draft } from "../rubrica/draft";
import type { ProfileDetail } from "../rubrica/types";
import type { TabProps } from "../shell/tabs/tab";
import "../shell/ProfileDetail.css";

const KEY = "c2VlZC1wdWJsaWMta2V5LWZvci10aGUtdGVzdHMtMDA=";

/** A WireGuard detail whose peer has a key the checks accept: the fixtures' one has the word «PUBLIC». */
export const withGoodKey = (detail: ProfileDetail): ProfileDetail =>
  detail.wireguard ? { ...detail, wireguard: { ...detail.wireguard, peers: detail.wireguard.peers.map((p) => ({ ...p, public_key: KEY })) } } : detail;

interface Options {
  groups?: string[];
  errors?: Record<string, string>;
  setSwitch?: TabProps["setSwitch"];
  lang?: "en" | "it";
}

/** One tab of the form on its own, with a draft it really changes and the writes of the switches given. */
function Harness({ Tab, detail, options }: { Tab: ComponentType<TabProps>; detail: ProfileDetail; options: Options }) {
  const [draft, setDraft] = useState<Draft>(() => toDraft(detail));
  return (
    <main>
      <Tab
        detail={detail}
        draft={draft}
        edit={(change) => setDraft(change)}
        errors={options.errors ?? {}}
        groups={options.groups ?? ["Lavoro", "Personale", "Clienti"]}
        settings={detail.settings}
        setSwitch={options.setSwitch ?? (() => undefined)}
      />
    </main>
  );
}

export const renderTab = (Tab: ComponentType<TabProps>, detail: ProfileDetail, options: Options = {}) =>
  render(
    <I18n lang={options.lang ?? "en"}>
      <Harness Tab={Tab} detail={detail} options={options} />
    </I18n>,
  );
