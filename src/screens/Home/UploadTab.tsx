import { Ok, Err, Result } from "ts-results-intraloop-fork";
import { invoke } from "@tauri-apps/api/tauri";
import { createSignal } from "solid-js";
import styles from "./home.module.css";
import { set_err } from "./signals";

export function UploadTab() {
  const [temp_pw, set_temp_pw] = createSignal("");
  const [pin, set_pin] = createSignal("");
  return (
    <div class={styles.push_container}>
      <p>Upload to sync server</p>
      <div class={styles.push_temp_pw_container}>
        <input
          class={styles.item_input}
          placeholder="Temporary password"
          type="password"
          onInput={(e) => {
            e.preventDefault();

            set_temp_pw(e.currentTarget.value);
          }}
        />
      </div>
      <div style={styles.item_buttons}>
        <button
          onClick={async () => {
            const pw = temp_pw();

            if (!pw) {
              return;
            }

            const res = await push(pw);

            if (res.err) {
              return set_err(res.val);
            }

            set_pin(res.val);

            console.log({ pin: res.val });
          }}
        >
          Upload
        </button>
      </div>
      {pin() && (
        <p>
          Your pin is {pin()}. Use the pin and your temporary password to
          download your passwords from the server.
        </p>
      )}
    </div>
  );
}

type PushResponse = {
  pin: string;
};

async function push(tempPw: string): Promise<Result<string, string>> {
  try {
    const resp = await invoke<PushResponse>("push_s", { tempPw });

    return Ok(resp.pin);
  } catch (err) {
    if (typeof err !== "string") {
      return Err(`An unknown error occurred: ${JSON.stringify(err)}`);
    }

    return Err(err);
  }
}
