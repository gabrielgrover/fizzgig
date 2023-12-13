import { Ok, Err, Result } from "ts-results-intraloop-fork";
import { invoke } from "@tauri-apps/api/tauri";
import { createEffect, createResource, createSignal, For } from "solid-js";
import styles from "./home.module.css";
import { Card } from "../../components";
import { writeText } from "@tauri-apps/api/clipboard";

const [err, set_err] = createSignal("");
const [should_fetch_labels, set_should_fetch_labels] = createSignal(false);

const NAV_TABS = ["All Items", "Export", "Upload", "Download"];
const [curr_tab, set_curr_tab] = createSignal("All Items");

export const Home = () => {
  createEffect(() => {
    if (err()) {
      console.log("ERR: ", err());
    }
  });

  return (
    <div class={styles.container}>
      <div class={styles.sidebar}>
        {/*
        <input
          class={styles.search_input}
          type="text"
          placeholder="Search..."
        />
				*/}
        <ul class={styles.navigation}>
          {NAV_TABS.map((tab_name) => (
            <li onClick={() => set_curr_tab(tab_name)}>{tab_name}</li>
          ))}
        </ul>
      </div>
      <div class={styles.content_container}>
        {curr_tab() === NAV_TABS[0] && <PasswordLabels />}
        {curr_tab() === NAV_TABS[1] && <ExportTab />}
        {curr_tab() === NAV_TABS[2] && <UploadTab />}
        {curr_tab() === NAV_TABS[3] && <DownloadTab />}
      </div>
    </div>
  );
};

function UploadTab() {
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

function DownloadTab() {
  const [temp_pw, set_temp_pw] = createSignal("");
  const [pin, set_pin] = createSignal("");

  return (
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
            const pull_result = await pull(temp_pw(), pin());

            if (pull_result.err) {
              console.error(pull_result.val);

              return set_err(pull_result.val);
            }

            console.log("Values: ", pull_result.val);
          }}
        >
          Download
        </button>
      </div>
    </div>
  );
}

function ExportTab() {
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

function PasswordLabels() {
  const [pw_labels, { refetch }] = createResource(() =>
    invoke<string[]>("list")
  );

  const [card_loading, set_card_loading] = createSignal("");

  createEffect(() => {
    if (typeof pw_labels.error !== "string") {
      set_err(`Unknown error occurred: ${pw_labels.error}`);
    }

    set_err(err);
    //console.log("ERROR: ", pw_labels.error);
  });

  createEffect(() => {
    if (should_fetch_labels()) {
      refetch();
      set_should_fetch_labels(false);
      set_card_loading("");
    }
  });

  return (
    <div class={styles.main}>
      <div class={styles.items}>
        {!pw_labels.error && (
          <For each={pw_labels()}>
            {(pw_label) => (
              <Card
                item_label={pw_label}
                onClick={async () => {
                  const pw = await get_pw(pw_label);

                  if (pw.err) {
                    set_err(pw.val);

                    return;
                  }

                  const res = await copy_to_clipboard(pw.val);

                  if (res.err) {
                    set_err(res.val);
                  }
                }}
                render_buttons={() =>
                  card_loading() === pw_label ? (
                    <div class={styles.spinner} />
                  ) : (
                    <div class={styles.item_buttons}>
                      <button
                        onClick={async () => {
                          set_card_loading(pw_label);
                          const res = await regen_pw(pw_label);

                          if (res.err) {
                            set_err(res.val);
                          }

                          set_card_loading("");
                        }}
                      >
                        Regen
                      </button>
                      <button
                        onClick={async () => {
                          set_card_loading(pw_label);
                          const res = await remove_password(pw_label);

                          if (res.err) {
                            set_err(res.val);
                          }

                          set_should_fetch_labels(true);
                        }}
                      >
                        Delete
                      </button>
                    </div>
                  )
                }
              />
            )}
          </For>
        )}
        <Card
          item_label="New entry"
          render_buttons={() => <NewEntryButton />}
        />
      </div>
    </div>
  );
}

function NewEntryButton() {
  const [should_edit, set_should_edit] = createSignal(false);
  const [input, set_input] = createSignal("");
  const [is_generating, set_is_generating] = createSignal(false);
  let input_ref: HTMLInputElement;

  createEffect(async () => {
    if (is_generating() && input()) {
      const result = await create_password(input());

      if (result.err) {
        set_err(result.val);
      }

      set_should_fetch_labels(true);
      set_is_generating(false);
      set_should_edit(false);
    }
  });

  return (
    <>
      {is_generating() && <div class={styles.spinner} />}
      {!should_edit() && !is_generating() && (
        <div class={styles.item_buttons}>
          <button
            onClick={() => {
              set_should_edit(true);
              input_ref.focus();
            }}
          >
            Generate
          </button>
        </div>
      )}
      {should_edit() && !is_generating() && (
        <div class={styles.item_input_container}>
          <input
            class={styles.item_input}
            placeholder="Password label"
            onInput={(e) => {
              e.preventDefault();
              set_input(e.currentTarget.value);
            }}
            onKeyPress={(e) => {
              if (e.key !== "Enter") {
                return;
              }

              set_is_generating(true);
              set_input("");
            }}
            onBlur={() => {
              set_should_edit(false);
            }}
            ref={(n) => {
              input_ref = n;
            }}
          />
        </div>
      )}
    </>
  );
}

async function create_password(label: string): Promise<Result<void, string>> {
  try {
    const pw = await generate_password();

    if (pw.err) {
      return Err(pw.val);
    }

    await invoke("add_entry", { entryName: label, val: pw.val });

    return Ok.EMPTY;
  } catch (err) {
    if (typeof err !== "string") {
      return Err(`An unknown error occurred: ${JSON.stringify(err)}`);
    }

    return Err(err);
  }
}

async function generate_password(): Promise<Result<string, string>> {
  try {
    const pw = await invoke<string>("generate_pw");

    return Ok(pw);
  } catch (err) {
    if (typeof err !== "string") {
      return Err(`An unknown error occurred: ${JSON.stringify(err)}`);
    }

    return Err(err);
  }
}

async function get_pw(label: string): Promise<Result<string, string>> {
  try {
    const pw = await invoke<string>("read_entry", { entryName: label });

    return Ok(pw);
  } catch (err) {
    const err_msg = JSON.stringify(err);

    return Err(err_msg);
  }
}

async function regen_pw(label: string): Promise<Result<void, string>> {
  try {
    await invoke("regen_pw", { entryName: label });

    return Ok.EMPTY;
  } catch (err) {
    if (typeof err !== "string") {
      return Err(`An unknown error occurred: ${JSON.stringify(err)}`);
    }

    return Err(err);
  }
}

async function remove_password(label: string): Promise<Result<void, string>> {
  try {
    await invoke("remove_entry", { entryName: label });

    return Ok.EMPTY;
  } catch (err) {
    if (typeof err !== "string") {
      return Err(`An unknown error occurred: ${JSON.stringify(err)}`);
    }

    return Err(err);
  }
}

async function copy_to_clipboard(text: string): Promise<Result<void, string>> {
  try {
    await writeText(text);

    return Ok.EMPTY;
  } catch (err) {
    const err_msg = JSON.stringify(err);

    return Err(err_msg);
  }
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

async function pull(tempPw: string, pin: string): Promise<Result<any, string>> {
  try {
    const values = await invoke<any>("pull", { tempPw, pin });

    return Ok(values);
  } catch (e) {
    if (typeof err !== "string") {
      return Err(`An unknown error occurred: ${JSON.stringify(e)}`);
    }

    return Err(err);
  }
}
