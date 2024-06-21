import { invoke } from "@tauri-apps/api/tauri";
import { createEffect, createSignal, For, onMount, Show } from "solid-js";
import styles from "./home.module.css";
import { Card } from "../../components";
import { writeText } from "@tauri-apps/api/clipboard";
import * as O from "fp-ts/Option";
import * as F from "fp-ts/function";
import * as E from "fp-ts/Either";

import {
  createLazyPwLoader,
  set_conflict_labels,
  set_err,
  sync_in_progress,
} from "./signals";
import { Err, Ok, Result } from "ts-results-intraloop-fork";

type Props = {
  on_conflict_click: () => void;
};

const { data: pw_data, load: load_pw_data } = createLazyPwLoader();

export function PasswordTab(props: Props) {
  const [card_loading, set_card_loading] = createSignal("");

  onMount(() => {
    load_pw_data();
  });

  return (
    <>
      <Show when={sync_in_progress()}>
        <p style={{ "padding-left": "20px" }}>
          Sync in progress. One moment please.
        </p>
      </Show>
      <Show when={!sync_in_progress()}>
        <div class={styles.main}>
          <div class={styles.items}>
            <For
              each={F.pipe(
                O.fromNullable(pw_data()),
                O.fold(
                  () => [],
                  E.fold(
                    (e) => {
                      set_err(e);
                      return [];
                    },
                    (i) => {
                      return i;
                    }
                  )
                )
              )}
            >
              {({ label: pw_label, has_conflict }) => {
                if (has_conflict) {
                  set_conflict_labels((prev) => prev.concat(pw_label));
                }

                return (
                  <Card
                    disable={has_conflict}
                    item_label={pw_label}
                    on_click={async () => {
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
                    render_buttons={() => {
                      if (has_conflict) {
                        return (
                          <div class={styles.conf_button}>
                            <button onClick={props.on_conflict_click}>
                              Resolve
                            </button>
                          </div>
                        );
                      }

                      if (card_loading() === pw_label) {
                        return <div class={styles.spinner} />;
                      }

                      return (
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

                              set_card_loading("");
                              load_pw_data();
                            }}
                          >
                            Delete
                          </button>
                        </div>
                      );
                    }}
                  />
                );
              }}
            </For>
            <Card
              item_label="New entry"
              render_buttons={() => <NewEntryButton />}
              disable
            />
          </div>
        </div>
      </Show>
    </>
  );
}

function NewEntryButton() {
  const [should_edit, set_should_edit] = createSignal(false);
  const [label, set_label] = createSignal("");
  const [is_generating, set_is_generating] = createSignal(false);
  const [pw, set_pw] = createSignal("");
  const [should_set, set_should_set] = createSignal(false);
  let input_ref: HTMLInputElement;

  createEffect(async () => {
    if (is_generating() && label() && !should_set()) {
      const result = await create_password(label());

      if (result.err) {
        set_err(result.val);
      }

      load_pw_data();
      set_is_generating(false);
      set_should_edit(false);
    }

    if (is_generating() && label() && pw()) {
      const result = await create_password(label(), pw());

      if (result.err) {
        set_err(result.val);
      }

      load_pw_data();
      set_is_generating(false);
      set_should_edit(false);
      set_should_set(false);
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
          <button
            onClick={() => {
              set_should_edit(true);
              set_should_set(true);
              input_ref.focus();
            }}
          >
            Set
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
              set_label(e.currentTarget.value);
            }}
            onKeyPress={(e) => {
              if (e.key !== "Enter") {
                return;
              }

              if (!should_set()) {
                set_is_generating(true);
              }
              //set_label("");
            }}
            onBlur={() => {
              if (!should_set()) {
                set_should_edit(false);
              }
            }}
            ref={(n) => {
              input_ref = n;
            }}
          />
        </div>
      )}
      {should_edit() && !is_generating() && label() && should_set() && (
        <div class={styles.item_input_container}>
          <input
            class={styles.item_input}
            placeholder="Password"
            onInput={(e) => {
              e.preventDefault();
              set_pw(e.currentTarget.value);
            }}
            onKeyPress={(e) => {
              if (e.key !== "Enter") {
                return;
              }

              set_is_generating(true);
            }}
            onBlur={() => {
              if (!should_set()) {
                set_should_edit(false);
              }
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

async function create_password(
  label: string,
  password?: string
): Promise<Result<void, string>> {
  try {
    const pw = password ? Ok(password) : await generate_password();

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
