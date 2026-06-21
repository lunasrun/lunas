import type { Router } from "../router";

declare global {
  interface Lunas {
    router: Router;
    afterMount: (callback: () => void) => void;
    afterUnmount: (callback: () => void) => void;
    watch: (items: unknown[], callback: () => void) => void;
  }

  var Lunas: Lunas;
}
