var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import { compile } from "lunas/compiler";
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
export function lunas() {
    // Map to store generated CSS for each .lun file
    const cssCodeMap = new Map();
    return {
        name: "vite-plugin-lunas",
        resolveId(id) {
            // Handle virtual CSS module for .lun files
            const [filename, query] = id.split("?", 2);
            if (filename.endsWith(".lun") && query === "style.css") {
                return id; // Mark as resolved for Vite
            }
        },
        transform(code, id) {
            return __awaiter(this, void 0, void 0, function* () {
                // Transform .lun files
                if (id.endsWith(".lun")) {
                    const result = compile(code);
                    if (result.css) {
                        // Store CSS for later retrieval
                        cssCodeMap.set(id, result.css);
                        return {
                            code: `import '${id}?style.css';\n${result.js}`, // Import virtual CSS module
                        };
                    }
                    return {
                        code: result.js,
                    };
                }
            });
        },
        load(id) {
            // Load the virtual CSS module for .lun files
            if (id.endsWith(".lun?style.css")) {
                const originalId = id.replace("?style.css", "");
                return cssCodeMap.get(originalId) || ""; // Return CSS or empty string
            }
        },
    };
}
