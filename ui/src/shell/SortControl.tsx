import { saveUiPrefs, type SortOrder } from "../app/uiState";
import { Segmented } from "../design/Segmented";
import { useToast } from "../design/Toast";
import { errorText, useT } from "../i18n";

/**
 * «By name · Most recent». The choice applies at once and is written to `state.toml` through `ui_state_set` (D32); if it
 * cannot be written the user is told it lasts only until the window closes, like for the language and the theme.
 */
export function SortControl({ value, onChange }: { value: SortOrder; onChange: (value: SortOrder) => void }) {
  const t = useT();
  const toast = useToast();
  const choose = (next: string) => {
    const sort = next as SortOrder;
    onChange(sort);
    saveUiPrefs({ sort }).catch((error: unknown) => {
      const { code, message } = (error ?? {}) as { code?: string; message?: string };
      toast(errorText(t, code ?? "gui_state_not_saved", message ?? ""), { tone: "error" });
    });
  };
  return (
    <Segmented
      label={t("sort.label")}
      value={value}
      onChange={choose}
      options={[
        { value: "name", label: t("sort.name") },
        { value: "recent", label: t("sort.recent") },
      ]}
    />
  );
}
