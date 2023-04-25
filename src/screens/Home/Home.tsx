import styles from "./home.module.css";
import { Card } from "../../components";
import { Ok, Err, Result } from "ts-results-intraloop-fork";
import { invoke } from "@tauri-apps/api/tauri";
import { createEffect, createResource, createSignal, For } from "solid-js";

const [err, set_err] = createSignal("");
const [should_fetch_labels, set_should_fetch_labels] = createSignal(false);

export const Home = () => {
  const [pw_labels, { refetch }] = createResource(get_pw_labels);

  createEffect(() => {
    if (should_fetch_labels()) {
      refetch();
      set_should_fetch_labels(false);
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
          <li>All Items</li>
          <li>Favorites</li>
          <li>Passwords</li>
          <li>Secure Notes</li>
          <li>Settings</li>
        </ul>
      </div>
      <div class={styles.main}>{render_pw_labels(pw_labels())}</div>
    </div>
  );
};

function render_pw_labels(pw_labels?: Result<string[], string>) {
  if (!pw_labels) {
    return null;
  }

  if (pw_labels.err) {
    set_err(pw_labels.val);

    console.log({ error: pw_labels.val });

    return null;
  }

  return (
    <div class={styles.items}>
      <For each={pw_labels.val}>
        {(pw_label) => (
          <Card
            item_label={pw_label}
            render_buttons={() => (
              <div class={styles.item_buttons}>
                <button>Edit</button>
                <button>Delete</button>
              </div>
            )}
          />
        )}
      </For>
      <Card item_label="New entry" render_buttons={() => <NewEntryButton />} />
    </div>
  );
}

function NewEntryButton() {
  const [should_edit, set_should_edit] = createSignal(false);
  const [input, set_input] = createSignal("");
  const [is_generating, set_is_generating] = createSignal(false);

  createEffect(async () => {
    if (is_generating()) {
      const result = await create_password(input());

      if (result.err) {
        set_err(result.val);
      }

      set_should_fetch_labels(true);
      set_is_generating(false);
    }
  });

  return (
    <>
      {is_generating() && <div class={styles.spinner} />}
      {!should_edit() && !is_generating() && (
        <div class={styles.item_buttons}>
          <button onClick={() => set_should_edit(true)}>Generate</button>
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
          />
        </div>
      )}
    </>
  );
}

async function get_pw_labels(): Promise<Result<string[], string>> {
  try {
    const labels = await invoke<string[]>("list");

    return Ok(labels);
  } catch (err) {
    if (typeof err !== "string") {
      return Err(`An unknown error occurred: ${JSON.stringify(err)}`);
    }

    return Err(err);
  }
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
