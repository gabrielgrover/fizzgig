import { Ok, Err, Result } from "ts-results-intraloop-fork";
import { invoke } from "@tauri-apps/api/tauri";
import styles from "./home.module.css";
import { set_err } from "./signals";

export function ExportTab() {
  return (
    <div class={styles.export_tab_container}>
      <div class={styles.export_cta}>
        <p>Export your passwords to a zipped file</p>
        <div style={styles.item_buttons}>
          <button
            onClick={async () => {
              const res = await export_ledger();

              if (res.err) {
                set_err(res.val);
              }
            }}
          >
            Export
          </button>
        </div>
      </div>
    </div>
  );
}

async function export_ledger(): Promise<Result<void, string>> {
  try {
    await invoke("export_ledger");

    return Ok.EMPTY;
  } catch (err) {
    if (typeof err !== "string") {
      return Err(`An unknown error occurred: ${JSON.stringify(err)}`);
    }

    return Err(err);
  }
}
