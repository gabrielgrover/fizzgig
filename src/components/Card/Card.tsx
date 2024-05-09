import { JSX } from "solid-js";
import styles from "./card.module.css";

type Props = {
  item_label: string;
  render_buttons: () => JSX.Element;
  on_click?: () => void;
  disable?: boolean;
};

export const Card = (props: Props) => {
  return (
    <div
      class={props.disable ? styles.item : styles.clickable_item}
      onClick={props.on_click}
    >
      <div class={styles.item_title}>{props.item_label}</div>
      {props.render_buttons()}
    </div>
  );
};
