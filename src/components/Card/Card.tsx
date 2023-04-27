import { JSX } from "solid-js";
import styles from "./card.module.css";

type Props = {
  item_label: string;
  render_buttons: () => JSX.Element;
  onClick?: () => void;
};

export const Card = (props: Props) => {
  return (
    <div
      class={props.onClick ? styles.clickable_item : styles.item}
      onClick={props.onClick}
    >
      <div class={styles.item_title}>{props.item_label}</div>
      {props.render_buttons()}
    </div>
  );
};
