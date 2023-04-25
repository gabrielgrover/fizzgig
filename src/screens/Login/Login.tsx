import { useNavigate } from "@solidjs/router";
import { invoke } from "@tauri-apps/api";
import * as TE from "fp-ts/TaskEither";
import * as O from "fp-ts/Option";
import * as F from "fp-ts/function";
import * as E from "fp-ts/Either";
import {
  Component,
  createEffect,
  createResource,
  createSignal,
} from "solid-js";
import { master_pass_signal } from "../../signals/master_pass";
import styles from "./login.module.css";

const COLLECTION_NAME = "First password ledger";

export const Login: Component = () => {
  const { set_master_pw, master_pw } = master_pass_signal;
  const [input, set_input] = createSignal("");
  const [data] = createResource(master_pw, start);
  const nav = useNavigate();

  createEffect(() => {
    const navigate_to_passswords_screen = O.map(E.map(() => nav("/home")));
    const get_possibly_null_data = O.fromNullable(data());

    F.pipe(get_possibly_null_data, navigate_to_passswords_screen);
  });

  return (
    <div class={styles.container}>
      <h1 class={styles.heading}>Enter your master password</h1>
      <input
        class={styles.password_input}
        onInput={(e) => {
          e.preventDefault();
          set_input(e.currentTarget.value);
        }}
        onKeyPress={async (e) => {
          if (e.key === "Enter") {
            set_master_pw(input());
          }
        }}
        placeholder="Password"
        type="password"
      />
      {F.pipe(
        O.fromNullable(data()),
        O.fold(
          () => null,
          E.fold(
            (err) => <p>Error: {err}</p>,
            () => null
          )
        )
      )}
    </div>
  );
};

function start(master_pw: string) {
  return TE.tryCatch(
    () =>
      invoke("open_collection", {
        ledgerName: COLLECTION_NAME,
        masterPw: master_pw,
      }),
    (err) => {
      if (typeof err === "string") {
        return err;
      }

      return `An unknown error occurred: ${JSON.stringify(err)}`;
    }
  )();
}
