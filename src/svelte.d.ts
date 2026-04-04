declare module "*.svelte" {
  import type { SvelteComponentTyped } from "svelte";

  export default class SvelteComponent<
    Props extends Record<string, unknown> = Record<string, never>,
    Events extends Record<string, unknown> = Record<string, never>,
    Slots extends Record<string, unknown> = Record<string, never>
  > extends SvelteComponentTyped<Props, Events, Slots> {}
}
