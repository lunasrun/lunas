import { Plugin } from "vite";
/**
 * Vite plugin for handling `.lun` files with custom compilation and CSS extraction.
 *
 * This plugin performs the following:
 * - Transforms `.lun` files by compiling them and extracting their CSS.
 * - Stores generated CSS in a map for each `.lun` file.
 * - Injects a virtual CSS module import into the transformed JavaScript code.
 * - Resolves and serves the virtual CSS module when requested by Vite.
 *
 * @returns {Plugin} A Vite plugin object for processing `.lun` files.
 */
export declare function lunas(): Plugin;
