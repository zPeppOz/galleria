import { Button } from "../design/Button";
import { icons } from "../design/icons";
import { useT } from "../i18n";
import "./CommandRow.css";

/** A command to copy and paste in a terminal. The window never runs it (D27). */
export function CommandRow({ command }: { command: string }) {
  const t = useT();
  return (
    <div className="command-row">
      <code className="command-row-text">{command}</code>
      <Button icon={icons.copy} onClick={() => void navigator.clipboard.writeText(command)}>
        {t("first.copy")}
      </Button>
    </div>
  );
}
