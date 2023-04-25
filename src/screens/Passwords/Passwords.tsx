import {
  Component,
  createEffect,
  createResource,
  createSignal,
  For,
} from "solid-js";
import { invoke } from "@tauri-apps/api/tauri";
import { writeText } from "@tauri-apps/api/clipboard";
import * as TE from "fp-ts/TaskEither";
import * as E from "fp-ts/Either";
import * as F from "fp-ts/function";
import * as A from "fp-ts/Array";
import * as O from "fp-ts/Option";
import styles from "./passwords.module.css";

const [password_label, set_password_label] = createSignal<string>();
const [err, set_err] = createSignal<string>();
const [added_passwords, set_added_passwords] = createSignal<string[]>([]);
const [loading, set_loading] = createSignal(false);

export const Passwords: Component = () => {
  const [input, set_input] = createSignal<string>("");
  const [save_pw_trigger, set_save_pw_trigger] = createSignal(false);
  const [show_text_input, set_show_text_input] = createSignal(false);
  const [list_of_stored_pws] = createResource(fetch_pw_list);

  createEffect(async () => {
    const label = password_label();
    const should_save = save_pw_trigger();
    const curr_added_passwords = added_passwords() ?? [];

    if (should_save && label) {
      F.pipe(
        await save_new_password(label),
        E.map(() => {
          set_save_pw_trigger(false);
          set_password_label("");
          set_added_passwords(curr_added_passwords.concat(label));
          set_show_text_input(false);
          set_loading(false);
        }),
        E.mapLeft(set_err)
      );
    }
  });

  return (
    <div>
      {render_password_list(list_of_stored_pws())}

      {show_text_input() ? (
        <>
          <input
            value={input()}
            class={styles.password_input}
            placeholder="Password Label"
            onInput={(e) => {
              e.preventDefault();
              set_input(e.currentTarget.value);
            }}
          />
          <button
            class={styles.button}
            onClick={() => {
              set_password_label(input());
              set_input("");
              set_save_pw_trigger(true);
              set_loading(true);
            }}
          >
            {loading() ? <div class={styles.spinner} /> : "Save"}
          </button>
        </>
      ) : (
        <button class={styles.button} onClick={() => set_show_text_input(true)}>
          Add Password
        </button>
      )}
      {render_err(err())}
    </div>
  );
};

function render_err(err_msg?: string) {
  if (!err_msg) {
    return null;
  }

  return <p>Error: {err_msg}</p>;
}

function render_password_list(password_labels?: E.Either<string, string[]>) {
  const _added_pw_list = F.pipe(
    O.fromNullable(added_passwords()),
    O.getOrElse<string[]>(() => [])
  );

  const merge_with_added_passwords = F.flow(
    A.concat(_added_pw_list),
    (all_pw_labels) => (
      <>
        <h2>My Passwords</h2>
        <p class={styles.tutorial_text}>
          Click label to copy password to your clipboard
        </p>
        <For each={all_pw_labels}>
          {(pw_label) => (
            <p
              class={styles.password_label}
              onClick={async () =>
                F.pipe(
                  await on_password_label_click(pw_label),
                  E.mapLeft(set_err)
                )
              }
            >
              {pw_label}
            </p>
          )}
        </For>
      </>
    )
  );

  const fetched_pws = F.pipe(
    O.fromNullable(password_labels),
    O.getOrElse(() => E.right<string, string[]>([])),
    E.fold(
      (err) => {
        set_err(err);
        return [];
      },
      (i) => i
    )
  );

  return F.pipe(fetched_pws, merge_with_added_passwords);
}

async function fetch_pw_list() {
  return TE.tryCatch(
    () =>
      invoke<string[]>("list").then((fetched_pws) => {
        console.log({ fetched_pws });

        return fetched_pws;
      }),
    (err) => {
      if (typeof err === "string") {
        return err;
      }

      return `An unknown error occurred: ${JSON.stringify(err)}`;
    }
  )();
}

async function save_new_password(label: string) {
  return F.pipe(
    generate_password(),
    TE.map((generated_password) => ({ generated_password, label })),
    TE.chain(({ generated_password, label }) =>
      add_password(label, generated_password)
    )
  )();
}

function generate_password() {
  return TE.tryCatch(
    () => invoke<string>("generate_pw"),
    (err) => {
      if (typeof err !== "string") {
        return `An unknown error occurred: ${JSON.stringify(err)}`;
      }

      return err;
    }
  );
}

function add_password(entryName: string, val: string) {
  return TE.tryCatch(
    () =>
      invoke("add_entry", {
        entryName,
        val,
      }),
    (err) => {
      if (typeof err === "string") {
        return err;
      }

      return `An unknown error occurred: ${JSON.stringify(err)}`;
    }
  );
}

async function on_password_label_click(label: string) {
  return F.pipe(get_pw(label), TE.chain(copy_to_clipboard))();
}

function get_pw(entryName: string) {
  return TE.tryCatch(
    () => invoke<string>("read_entry", { entryName }),
    (err) => {
      if (typeof err !== "string") {
        return `An unknown error occurred: ${JSON.stringify(err)}`;
      }

      return err;
    }
  );
}

function copy_to_clipboard(text: string) {
  return TE.tryCatch(
    () => writeText(text),
    (err) => {
      if (typeof err !== "string") {
        return `An unknown error occurred: ${JSON.stringify(err)}`;
      }

      return err;
    }
  );
}
