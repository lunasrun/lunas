import { glob } from "glob";
import path from "path";
import slugify from "slugify";
/**
 * Throw an error if the slug already exists.
 * @param base - The slug candidate.
 * @param existing - A set of slugs already used.
 */
function uniqueSlug(base, existing) {
    if (existing.has(base)) {
        throw new Error(`Route collision detected: "${base}"`);
    }
    existing.add(base);
    return base;
}
/**
 * Scan for .lun files and produce an array of route definitions.
 * @param absDir - Absolute path to the pages directory.
 * @param projectRoot - Absolute path to project root.
 * @returns A stringified array of route objects.
 */
function generateRoutes(absDir, projectRoot) {
    // Build glob pattern for .lun files
    const pattern = path.posix.join(absDir.replace(/\\/g, "/"), "**/*.lun");
    const files = glob.sync(pattern, { nodir: true });
    const seen = new Set();
    const routes = files.map((file) => {
        // Compute path relative to pagesDir and remove extension
        const relToPages = path
            .relative(absDir, file)
            .slice(0, -4)
            .split(path.sep)
            .join("-");
        // Slugify the path segment
        const rawSlug = slugify(relToPages, {
            lower: true,
            strict: true,
        });
        // Only exact "index" maps to root; other "-index" remain unchanged
        const slug = rawSlug === "index" ? "" : rawSlug;
        // Ensure uniqueness (will throw on collision)
        const safeName = uniqueSlug(slug, seen);
        // Build the route path
        const routePath = safeName === "" ? `/` : `/${safeName}`;
        // Build import path relative to project root
        const relImportPath = path
            .relative(projectRoot, file)
            .split(path.sep)
            .join("/");
        return `{ path: ${JSON.stringify(routePath)}, component: () => import(${JSON.stringify(`./${relImportPath}`)}) }`;
    });
    return `[${routes.join(",")}]`;
}
/**
 * Vite plugin for automatic routing based on .lun files.
 * @param options.pagesDir - Relative path to the directory containing page components.
 */
export function lunasAutoRoutingPlugin(options) {
    let projectRoot;
    return {
        name: "vite-plugin-lunas-auto-routing",
        enforce: "pre",
        /**
         * Capture the resolved project root.
         */
        configResolved(config) {
            projectRoot = config.root;
        },
        /**
         * Resolve the virtual module ID for generated routes.
         */
        resolveId(source) {
            if (source === "virtual:generated-routes") {
                return source;
            }
            return null;
        },
        /**
         * Load and return the virtual module content.
         */
        load(id) {
            if (id === "virtual:generated-routes") {
                const absPagesDir = path.resolve(projectRoot, options.pagesDir);
                if (!absPagesDir.startsWith(projectRoot)) {
                    throw new Error(`Security error: pagesDir must be inside the project root: ${absPagesDir}`);
                }
                const routesArray = generateRoutes(absPagesDir, projectRoot);
                return `export const routes = ${routesArray};`;
            }
            return null;
        },
    };
}
