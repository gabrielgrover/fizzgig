import styles from "./conflicttab.module.css";
import { createLazyPwLoader, set_err } from "./signals";
import * as F from "fp-ts/function";
import * as E from "fp-ts/Either";
import * as O from "fp-ts/Option";
import * as A from "fp-ts/Array";
import * as TE from "fp-ts/TaskEither";
import {
  createEffect,
  createResource,
  createSignal,
  For,
  onMount,
  Show,
  Suspense,
} from "solid-js";
import { invoke } from "@tauri-apps/api";

const { data, load } = createLazyPwLoader();

export function ConflictTab() {
  const [revealed, set_revealed] = createSignal("");
  const [revealed_data] = createResource(revealed, reveal_conflict);
  const [reveal_err, set_reveal_err] = createSignal("");
  const [selected, set_selected] = createSignal("");
  const [resolve_err, set_resolve_err] = createSignal("");

  onMount(() => {
    load();
  });

  createEffect(() => {
    reveal_err() && console.error(reveal_err());
    resolve_err() && console.error(resolve_err());
  });

  return (
    <div class={styles.conflict_container}>
      <For each={get_pw_conflicts()}>
        {(label) => {
          return (
            <div class={styles.card}>
              <Show when={revealed()}>
                <h2>Resolve conflict</h2>
                <p>Select the correct password for {label}</p>
                <Suspense
                  fallback={
                    <div class={styles.spinner_container}>
                      <div class={styles.spinner} />
                    </div>
                  }
                >
                  <div class={styles.radio_container}>
                    {F.pipe(
                      O.fromNullable(revealed_data()),
                      O.fold(
                        () => null,
                        E.fold(
                          (e) => {
                            set_reveal_err(e);
                            return null;
                          },
                          (conf_pair) => (
                            <>
                              <div class={styles.input_container}>
                                <input
                                  value={conf_pair.local_pw}
                                  onChange={(evt) =>
                                    set_selected(evt.target.value)
                                  }
                                  checked={selected() === conf_pair.local_pw}
                                  id="current-pw"
                                  type="radio"
                                />
                                <label
                                  for="current-pw"
                                  onClick={() =>
                                    set_selected(conf_pair.local_pw)
                                  }
                                >
                                  {conf_pair.local_pw} (local)
                                </label>
                              </div>
                              <div class={styles.input_container}>
                                <input
                                  value={conf_pair.remote_pw}
                                  onChange={(evt) =>
                                    set_selected(evt.target.value)
                                  }
                                  checked={selected() === conf_pair.remote_pw}
                                  id="incoming-pw"
                                  type="radio"
                                />
                                <label
                                  for="incoming-pw"
                                  onClick={() =>
                                    set_selected(conf_pair.remote_pw)
                                  }
                                >
                                  {conf_pair.remote_pw} (remote)
                                </label>
                              </div>
                              <div class={styles.item_buttons}>
                                <button
                                  onClick={F.pipe(
                                    O.fromNullable(selected()),
                                    TE.fromOption(() => ""),
                                    TE.chain((selected_pw) => {
                                      return resolve(
                                        label,
                                        selected_pw === conf_pair.local_pw
                                      );
                                    }),
                                    TE.mapLeft(set_resolve_err)
                                  )}
                                >
                                  Submit
                                </button>
                              </div>
                            </>
                          )
                        )
                      )
                    )}
                  </div>
                </Suspense>
              </Show>
              <Show when={!revealed()}>
                <div class={styles.card_title}>{label}</div>
                <div class={styles.item_buttons}>
                  <button
                    onClick={() => {
                      set_revealed(label);
                    }}
                  >
                    Reveal conflict
                  </button>
                </div>
              </Show>
            </div>
          );
        }}
      </For>
    </div>
  );
}

function get_pw_conflicts() {
  return F.pipe(
    O.fromNullable(data()),
    O.fold(
      () => [],
      E.fold(
        (err) => {
          set_err(err);
          return [];
        },
        F.flow(
          A.filter((pw_meta_data) => pw_meta_data.has_conflict),
          A.map((pw_meta_data) => pw_meta_data.label)
        )
      )
    )
  );
}

function reveal_conflict(label: string) {
  if (label.length === 0) {
    return null;
  }

  return TE.tryCatch(
    async () => {
      const r = await invoke<{ local_pw: string; remote_pw: string }>(
        "get_conf_pair",
        {
          entryName: label,
        }
      );

      return r;
    },
    (err) => {
      if (typeof err === "string") {
        return err;
      }

      return `An unknown error occurred: ${JSON.stringify(err, null, 2)}`;
    }
  )();
}

function resolve(entry_name: string, keep_original: boolean) {
  return TE.tryCatch(
    async () => {
      const r = await invoke<void>("resolve_conflict", {
        entryName: entry_name,
        keepOriginal: keep_original,
      });

      return r;
    },
    (err) => {
      if (typeof err === "string") {
        return err;
      }

      return `An unknown error occurred: ${JSON.stringify(err, null, 2)}`;
    }
  );
}
