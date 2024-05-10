import { Ok, Err, Result } from "ts-results-intraloop-fork";
import { invoke } from "@tauri-apps/api/tauri";
import { createSignal, Show } from "solid-js";
import styles from "./home.module.css";
import {
  createLazyPwLoader,
  err,
  set_err,
  set_sync_in_progress,
  sync_in_progress,
} from "./signals";

const { load: load_pw_data } = createLazyPwLoader();

export function DownloadTab() {
  const [temp_pw, set_temp_pw] = createSignal("");
  const [pin, set_pin] = createSignal("");

  return (
    <>
      <Show when={sync_in_progress()}>
        <p style={{ "padding-left": "20px" }}>
          Sync in progress. One moment please.
        </p>
      </Show>
      <Show when={!sync_in_progress()}>
        <div class={styles.pull_container}>
          <p>Download from sync server</p>
          <div class={styles.push_temp_pw_container}>
            <input
              class={styles.item_input}
              placeholder="Pin"
              onInput={(e) => set_pin(e.currentTarget.value)}
            />
          </div>
          <div class={styles.push_temp_pw_container}>
            <input
              class={styles.item_input}
              placeholder="Temporary password"
              type="password"
              onInput={(e) => set_temp_pw(e.currentTarget.value)}
            />
          </div>
          <div style={styles.item_buttons}>
            <button
              onClick={async () => {
                set_sync_in_progress(true);
                const pull_result = await pull(temp_pw(), pin());
                set_sync_in_progress(false);

                if (pull_result.err) {
                  return set_err(pull_result.val);
                }
              }}
            >
              Download
            </button>
          </div>
        </div>
      </Show>
    </>
  );
}

async function pull(tempPw: string, pin: string): Promise<Result<any, string>> {
  try {
    const values = await invoke<any>("pull", { tempPw, pin });

    load_pw_data();

    return Ok(values);
  } catch (e) {
    if (typeof err !== "string") {
      return Err(`An unknown error occurred: ${JSON.stringify(e)}`);
    }

    return Err(err);
  }
}
