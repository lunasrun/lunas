#!/usr/bin/env node
"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const degit_1 = __importDefault(require("degit"));
const fs_1 = require("fs");
const promises_1 = require("fs/promises");
const enquirer_1 = require("enquirer");
const path_1 = __importDefault(require("path"));
(() => __awaiter(void 0, void 0, void 0, function* () {
    try {
        // Template repository
        const repo = "lunasrun/lunas-template";
        // Prompt for project name
        const { project } = yield (0, enquirer_1.prompt)({
            type: "input",
            name: "project",
            message: "Project name:",
            initial: "your-lunas-project-name",
        });
        const targetDir = project.trim();
        // Check if directory already exists
        if ((0, fs_1.existsSync)(targetDir)) {
            console.error(`❌ Directory "${targetDir}" already exists.`);
            process.exit(1);
        }
        console.log(`📦 Initializing project in "${targetDir}"...`);
        const emitter = (0, degit_1.default)(repo);
        yield (0, promises_1.mkdir)(targetDir, { recursive: true });
        yield emitter.clone(targetDir);
        yield renameFiles(targetDir);
        console.log("✅ Project initialized.");
        console.log("👉 Next steps:");
        console.log(`   cd ${targetDir}`);
        console.log("   npm install");
        console.log("   npm run dev");
    }
    catch (err) {
        console.error("❌ Failed to initialize project:", err);
        process.exit(1);
    }
}))();
function renameFiles(projectName) {
    return __awaiter(this, void 0, void 0, function* () {
        const filesToRename = ["README.md", "index.html", "package.json"];
        for (const file of filesToRename) {
            const filePath = path_1.default.join(projectName, file);
            let content;
            try {
                content = yield (0, promises_1.readFile)(filePath, "utf8");
            }
            catch (error) {
                if (typeof error === "object" &&
                    error !== null &&
                    error.code === "ENOENT")
                    continue; // If the file does not exist, ignore it
                throw error;
            }
            const updatedContent = content.replace(/__PROJECT_NAME__/g, projectName);
            yield (0, promises_1.writeFile)(filePath, updatedContent, "utf8");
        }
    });
}
