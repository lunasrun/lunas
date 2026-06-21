import type { Plugin } from "vite";
/**
 * Vite plugin for automatic routing based on .lun files.
 * @param options.pagesDir - Relative path to the directory containing page components.
 */
export declare function lunasAutoRoutingPlugin(options: {
    pagesDir: string;
}): Plugin;
