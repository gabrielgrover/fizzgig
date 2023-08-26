import { Ok, Err, Result } from "ts-results-intraloop-fork";
import { invoke } from "@tauri-apps/api/tauri";
import { createEffect, createResource, createSignal, For } from "solid-js";
import styles from "./home.module.css";
import { Card } from "../../components";
import { writeText } from "@tauri-apps/api/clipboard";

const [err, set_err] = createSignal("");
const [should_fetch_labels, set_should_fetch_labels] = createSignal(false);

const NAV_TABS = ["All Items", "Export"];
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
        <input
          class={styles.search_input}
          type="text"
          placeholder="Search..."
        />
        <ul class={styles.navigation}>
          {NAV_TABS.map((tab_name) => (
            <li onClick={() => set_curr_tab(tab_name)}>{tab_name}</li>
          ))}
        </ul>
      </div>
      {curr_tab() === NAV_TABS[0] && <PasswordLabels />}
      {curr_tab() === NAV_TABS[1] && <ExportTab />}
    </div>
  );
};

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

      <div class={styles.export_cta}>
        <p>Push to sync server</p>
        <div style={styles.item_buttons}>
          <button
            onClick={async () => {
              const res = await push();

              if (res.err) {
                set_err(res.val);
              }
            }}
          >
            Push
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

async function push(): Promise<Result<void, string>> {
  try {
    await invoke("push");

    return Ok.EMPTY;
  } catch (err) {
    if (typeof err !== "string") {
      return Err(`An unknown error occurred: ${JSON.stringify(err)}`);
    }

    return Err(err);
  }
}
