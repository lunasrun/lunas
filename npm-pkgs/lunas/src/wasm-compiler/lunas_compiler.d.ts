/* tslint:disable */
/* eslint-disable */
export function compile(lunas_code: string, engine_path?: string | null): LunasCompilerOutput;
export class LunasCompilerOutput {
  private constructor();
  free(): void;
  readonly js: string;
  readonly css: string | undefined;
}
