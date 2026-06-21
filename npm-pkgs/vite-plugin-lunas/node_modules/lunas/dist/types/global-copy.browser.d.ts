declare global {
  interface Lunas {
    afterMount: (callback: () => void) => void;
    afterUnmount: (callback: () => void) => void;
    watch: (items: unknown[], callback: () => void) => void;
  }

  var Lunas: Lunas;
}

export {};
