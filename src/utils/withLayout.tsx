import { Component } from "solid-js";
import { Layout } from "../components/Layout";

export const withLayout = (C: Component) => () =>
  (
    <Layout>
      <C />
    </Layout>
  );
