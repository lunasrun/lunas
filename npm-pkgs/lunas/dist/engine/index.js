"use strict";
/** biome-ignore-all lint/suspicious/noExplicitAny: user inputs are unpredictable, so accepting `any` is necessary to handle arbitrary data. */
/** biome-ignore-all lint/suspicious/noPrototypeBuiltins: ensures compatibility with environments where Object.prototype methods may be shadowed or customized. */
/** biome-ignore-all lint/style/noNonNullAssertion: earlier validation guarantees non-null values, but TypeScript cannot infer this fact. */
Object.defineProperty(exports, "__esModule", { value: true });
exports.reactive = exports.$$lunasCreateNonReactive = exports.$$createLunasElement = exports.$$lunasReplaceAttr = exports.$$lunasReplaceText = exports.$$lunasEscapeHtml = exports.$$lunasInitComponent = exports.valueObj = void 0;
var BlockType;
(function (BlockType) {
    BlockType["IF"] = "IF";
    BlockType["FOR"] = "FOR";
})(BlockType || (BlockType = {}));
class valueObj {
    constructor(initialValue, componentObj, componentSymbol, symbolIndex = [0]) {
        // Dependencies map: key is a symbol, value is a tuple of [LunasComponentState, number[]]
        this.dependencies = {};
        this._v = initialValue;
        if (componentSymbol && componentObj) {
            this.dependencies[componentSymbol] = [componentObj, symbolIndex];
        }
        // If the initial value is an object (and not null), wrap it with a Proxy
        if (typeof initialValue === "object" && initialValue !== null) {
            this.proxy = this.createProxy(initialValue);
        }
        else {
            this.proxy = initialValue;
        }
    }
    set v(v) {
        if (this._v === v)
            return;
        this._v = v;
        // If the new value is an object, wrap it with a Proxy
        if (typeof v === "object" && v !== null) {
            this.proxy = this.createProxy(v);
        }
        else {
            this.proxy = v;
        }
        this.triggerUpdate();
    }
    get v() {
        return this.proxy;
    }
    // Triggers an update for all dependencies
    triggerUpdate() {
        for (const key of Object.getOwnPropertySymbols(this.dependencies)) {
            const [componentObj, symbolIndex] = this.dependencies[key];
            bitOrAssign(componentObj.valUpdateMap, symbolIndex);
            if (!componentObj.updatedFlag && componentObj.__lunas_update) {
                Promise.resolve().then(componentObj.__lunas_update.bind(componentObj));
                componentObj.updatedFlag = true;
            }
        }
    }
    // Creates a Proxy recursively to detect changes in nested objects and arrays
    createProxy(target) {
        const self = this;
        // If target is not an object or is null, return it directly
        if (typeof target !== "object" || target === null) {
            return target;
        }
        return new Proxy(target, {
            get(target, prop, receiver) {
                const value = Reflect.get(target, prop, receiver);
                // Wrap array mutation methods to trigger update
                if (Array.isArray(target) &&
                    typeof value === "function" &&
                    [
                        "push",
                        "pop",
                        "shift",
                        "unshift",
                        "splice",
                        "sort",
                        "reverse",
                    ].includes(prop.toString())) {
                    return (...args) => {
                        const result = value.apply(target, args);
                        self.triggerUpdate();
                        return result;
                    };
                }
                // If the value is an object, return a Proxy for it (recursive wrapping)
                if (typeof value === "object" && value !== null) {
                    return self.createProxy(value);
                }
                return value;
            },
            set(target, prop, value, receiver) {
                const oldVal = target[prop];
                if (oldVal === value)
                    return true;
                // If the new value is an object, wrap it with a Proxy before setting it
                const newValue = typeof value === "object" && value !== null
                    ? self.createProxy(value)
                    : value;
                const result = Reflect.set(target, prop, newValue, receiver);
                self.triggerUpdate();
                return result;
            },
        });
    }
    // Adds a dependency and returns a removal function
    addDependency(componentObj, symbolIndex) {
        this.dependencies[componentObj.compSymbol] = [componentObj, symbolIndex];
        return {
            removeDependency: () => {
                delete this.dependencies[componentObj.compSymbol];
            },
        };
    }
    addToCurrentDependency(componentObj, symbolIndex) {
        const currentDep = Object.getOwnPropertySymbols(this.dependencies).find((key) => key === componentObj.compSymbol);
        const currentSymbolIndex = currentDep
            ? this.dependencies[currentDep][1]
            : null;
        if (currentSymbolIndex) {
            const maskedSymbolIndex = bitCombine(currentSymbolIndex, symbolIndex);
            this.dependencies[componentObj.compSymbol] = [
                componentObj,
                maskedSymbolIndex,
            ];
        }
        else {
            this.dependencies[componentObj.compSymbol] = [componentObj, symbolIndex];
        }
    }
}
exports.valueObj = valueObj;
const $$lunasInitComponent = function (args = {}, inputs = []) {
    this.updatedFlag = false;
    this.valUpdateMap = [0];
    this.blkUpdateMap = {};
    this.currentVarBitGen = bitArrayGenerator();
    this.isMounted = false;
    this.ifBlocks = {};
    this.ifBlockStates = {};
    this.compSymbol = Symbol();
    this.resetDependecies = [];
    this.refMap = [];
    this.updateComponentFuncs = [[], []];
    this.updateBlockFuncs = [];
    this.forBlocks = {};
    this.__lunas_after_mount = () => { };
    this.__lunas_destroy = () => { };
    for (const key of inputs) {
        const arg = args[key];
        if (arg instanceof valueObj) {
            const { removeDependency } = arg.addDependency(this, this.currentVarBitGen.next().value);
            this.resetDependecies.push(removeDependency);
        }
        else {
            this.currentVarBitGen.next().value;
        }
    }
    const getElm = function (location) {
        return getNestedArrayValue(this.refMap, location);
    }.bind(this);
    const setImportVars = function (items) {
        for (const item of items) {
            if (item instanceof valueObj) {
                const { removeDependency } = item.addDependency(this, this.currentVarBitGen.next().value);
                this.resetDependecies.push(removeDependency);
            }
            else if (isReactive(item)) {
                const { removeDependency } = item.addDependency(this, this.currentVarBitGen.next().value);
                this.resetDependecies.push(removeDependency);
            }
            else {
                this.currentVarBitGen.next().value;
            }
        }
    }.bind(this);
    const componentElementSetter = function (innerHtml, topElmTag, topElmAttr = {}) {
        this.internalElement = {
            innerHtml,
            topElmTag,
            topElmAttr,
        };
    }.bind(this);
    const applyEnhancement = function (enhancementFunc) {
        this.__lunas_apply_enhancement = enhancementFunc;
    }.bind(this);
    const setAfterMount = function (afterMount) {
        this.__lunas_after_mount = afterMount;
    }.bind(this);
    const setAfterUnmount = function (afterUnmount) {
        this.__lunas_destroy = afterUnmount;
    }.bind(this);
    const mount = function (elm) {
        if (this.isMounted)
            throw new Error("Component is already mounted");
        elm.innerHTML = `<${this.internalElement.topElmTag} ${Object.keys(this.internalElement.topElmAttr)
            .map((key) => `${key}="${this.internalElement.topElmAttr[key]}"`)
            .join(" ")}>${this.internalElement.innerHtml}</${this.internalElement.topElmTag}>`;
        this.componentElm = elm.firstElementChild;
        this.__lunas_apply_enhancement();
        this.__lunas_after_mount();
        this.isMounted = true;
        _updateComponent(() => { });
        return this;
    }.bind(this);
    const insert = function (elm, anchor) {
        if (this.isMounted)
            throw new Error("Component is already mounted");
        this.componentElm = _createDomElementFromLunasElement(this.internalElement);
        elm.insertBefore(this.componentElm, anchor);
        this.__lunas_apply_enhancement();
        this.__lunas_after_mount();
        this.isMounted = true;
        _updateComponent(() => { });
        return this;
    }.bind(this);
    const __unmount = function () {
        if (!this.isMounted)
            throw new Error("Component is not mounted");
        this.componentElm.remove();
        this.isMounted = false;
        this.resetDependecies.forEach((r) => r());
        this.__lunas_destroy();
    }.bind(this);
    const _updateComponent = function (updateFunc) {
        this.__lunas_update = (() => {
            if (!this.updatedFlag)
                return;
            this.updateComponentFuncs[0].forEach((f) => f === null || f === void 0 ? void 0 : f());
            const forBlockIds = this.updateBlockFuncs.map((blk) => blk.name);
            const funcsSnapshot = {};
            for (const id of forBlockIds) {
                funcsSnapshot[id] = this.updateBlockFuncs
                    .find((blk) => blk.name === id)
                    .updateFuncs.slice();
            }
            for (const oldKey of forBlockIds) {
                const funcs = this.updateBlockFuncs.find((blk) => blk.name === oldKey).updateFuncs;
                // console.log(`key ${oldKey}`);
                for (const func of funcs) {
                    if (funcsSnapshot[oldKey].indexOf(func) !== -1) {
                        func();
                    }
                }
            }
            this.updateComponentFuncs[1].forEach((f) => f === null || f === void 0 ? void 0 : f());
            updateFunc.call(this);
            this.updatedFlag = false;
            this.valUpdateMap = [0];
            this.blkUpdateMap = {};
        }).bind(this);
    }.bind(this);
    const createReactive = function (v) {
        return new valueObj(v, this, this.compSymbol, this.currentVarBitGen.next().value);
    }.bind(this);
    const createIfBlock = function (ifBlocks, indices) {
        for (const [getName, lunasElement, condition, postRender, ifCtxUnderFor, forCtx, depBit, [mapOffset, mapLength], [parentElementIndex, refElementIndex], fragments,] of ifBlocks) {
            const ifBlockId = typeof getName === "function" ? getName() : getName;
            setNestedArrayValue(this.refMap, mapOffset, undefined);
            this.ifBlocks[ifBlockId] = {
                renderer: ((mapOffset, _mapLength) => {
                    const componentElm = _createDomElementFromLunasElement(lunasElement());
                    const parentElement = getNestedArrayValue(this.refMap, parentElementIndex);
                    const refElement = getNestedArrayValue(this.refMap, refElementIndex);
                    parentElement.insertBefore(componentElm, refElement !== null && refElement !== void 0 ? refElement : null);
                    setNestedArrayValue(this.refMap, mapOffset, componentElm);
                    postRender();
                    if (fragments) {
                        createFragments(fragments, [...ifCtxUnderFor, ifBlockId]);
                    }
                    this.ifBlockStates[ifBlockId] = true;
                    this.blkUpdateMap[ifBlockId] = true;
                    Object.values(this.ifBlocks).forEach((blk) => {
                        if (blk.context.includes(ifBlockId)) {
                            blk.condition() && blk.renderer();
                        }
                    });
                }).bind(this, mapOffset, mapLength),
                context: ifCtxUnderFor.map((ctx) => indices ? `${ctx}-${indices}` : ctx),
                condition,
                forBlk: forCtx.length ? forCtx[forCtx.length - 1] : null,
                cleanup: [],
                childs: [],
                nextForBlocks: [],
            };
            ifCtxUnderFor.forEach((ctx) => {
                const parentBlockName = indices ? `${ctx}-${indices}` : ctx;
                this.ifBlocks[parentBlockName].childs.push(ifBlockId);
            });
            const updateFunc = (() => {
                if (bitAnd(this.valUpdateMap, depBit)) {
                    const shouldRender = condition();
                    const rendered = !!this.ifBlockStates[ifBlockId];
                    const parentRendered = ifCtxUnderFor.every((ctx) => this.ifBlockStates[indices ? `${ctx}-${indices}` : ctx]);
                    if (shouldRender && !rendered && parentRendered) {
                        this.ifBlocks[ifBlockId].renderer();
                        this.ifBlocks[ifBlockId].nextForBlocks.forEach((blkName) => {
                            const forBlk = this.forBlocks[blkName];
                            if (forBlk) {
                                forBlk.renderer();
                            }
                        });
                    }
                    else if (!shouldRender && rendered) {
                        const ifBlkElm = getNestedArrayValue(this.refMap, mapOffset);
                        ifBlkElm.remove();
                        if (typeof mapOffset === "number") {
                            this.refMap.fill(undefined, mapOffset, mapOffset + mapLength);
                        }
                        else {
                            for (let i = 0; i < mapLength; i++) {
                                const copiedMapOffset = [...mapOffset];
                                copiedMapOffset[0] += i;
                                setNestedArrayValue(this.refMap, copiedMapOffset, undefined);
                            }
                        }
                        // delete all childs of if here
                        this.ifBlocks[ifBlockId].childs.forEach((child) => {
                            if (this.ifBlockStates[child] === true) {
                                this.ifBlockStates[child] = false;
                                console.log(`marked ifBlock ${child} as false`);
                            }
                            // if (this.forBlocks[child] === true) {
                            //   this.ifBlockStates[child] = false;
                            // }
                        });
                        // console.log(`delete block ${ifBlockId}`);
                        delete this.ifBlockStates[ifBlockId];
                        [ifBlockId, ...this.ifBlocks[ifBlockId].childs].forEach((child) => {
                            if (this.ifBlocks[child]) {
                                this.ifBlocks[child].cleanup.forEach((f) => f());
                                this.ifBlocks[child].cleanup = [];
                            }
                        });
                    }
                }
            }).bind(this);
            if (!this.updateBlockFuncs.find((blk) => blk.name === ifBlockId)) {
                this.updateBlockFuncs.push({
                    name: ifBlockId,
                    type: BlockType.IF,
                    updateFuncs: [],
                });
            }
            this.updateBlockFuncs
                .find((blk) => blk.name === ifBlockId)
                .updateFuncs.push(updateFunc);
            const latestForName = forCtx[forCtx.length - 1];
            if (latestForName) {
                const cleanUpFunc = (() => {
                    this.updateBlockFuncs.find((blk) => blk.name === ifBlockId).updateFuncs = [];
                }).bind(this);
                const popedIndices = copyAndPopArray(indices);
                const latestForNameWithIndices = popedIndices.length > 0
                    ? `${latestForName}-${popedIndices}`
                    : latestForName;
                this.forBlocks[latestForNameWithIndices].cleanUp.push(cleanUpFunc);
            }
            if (ifCtxUnderFor.length === 0) {
                condition() && this.ifBlocks[ifBlockId].renderer();
            }
            else {
                const parentBlockName = indices
                    ? `${ifCtxUnderFor[ifCtxUnderFor.length - 1]}-${indices}`
                    : ifCtxUnderFor[ifCtxUnderFor.length - 1];
                if (this.ifBlockStates[parentBlockName] &&
                    condition() &&
                    !this.ifBlockStates[ifBlockId]) {
                    this.ifBlocks[ifBlockId].renderer();
                }
            }
            if (this.forBlocks[forCtx[forCtx.length - 1]]) {
                this.forBlocks[forCtx[forCtx.length - 1]].cleanUp.push(() => {
                    [ifBlockId, ...this.ifBlocks[ifBlockId].childs].forEach((child) => {
                        if (this.ifBlocks[child]) {
                            this.ifBlocks[child].cleanup.forEach((f) => f());
                            this.ifBlocks[child].cleanup = [];
                        }
                    });
                });
            }
        }
        this.blkUpdateMap = {};
    }.bind(this);
    const renderIfBlock = function (name) {
        if (!this.ifBlocks[name])
            return;
        this.ifBlocks[name].renderer();
    }.bind(this);
    const getElmRefs = function (ids, preserveId, refLocation = 0) {
        const boolMap = bitMapToBoolArr(preserveId);
        ids.forEach(function (id, index) {
            const e = document.getElementById(id);
            if (boolMap[index]) {
                e.removeAttribute("id");
            }
            const newRefLocation = addNumberToArrayInitial(refLocation, index);
            setNestedArrayValue(this.refMap, newRefLocation, e);
        }.bind(this));
    }.bind(this);
    const addEvListener = function (args) {
        for (const [elmIdx, evName, evFunc] of args) {
            const target = getNestedArrayValue(this.refMap, elmIdx);
            target.addEventListener(evName, evFunc);
        }
    }.bind(this);
    const createForBlock = function (forBlocksConfig, indices) {
        for (const config of forBlocksConfig) {
            const [getName, renderItem, getDataArray, afterRenderHook, ifCtxUnderFor, forCtx, prevIfCtx, updateFlag, parentIndices, [mapOffset, mapLength], [parentElementIndex, refElementIndex], fragmentFunc,] = config;
            const forBlockId = typeof getName === "function" ? getName() : getName;
            const blkName = indices ? `${prevIfCtx}-${indices}` : prevIfCtx;
            if (prevIfCtx && this.ifBlocks[blkName]) {
                this.ifBlocks[blkName].nextForBlocks.push(forBlockId);
            }
            // TODO: Review the necessity of this block
            forCtx.forEach((ctx) => {
                const allCtxPatterns = [];
                const copiedIndices = indices ? indices.slice() : [];
                while (true) {
                    allCtxPatterns.push(copiedIndices.length > 0 ? `${ctx}-${copiedIndices}` : ctx);
                    copiedIndices.pop();
                    if (!copiedIndices || copiedIndices.length === 0) {
                        break;
                    }
                }
                allCtxPatterns.forEach((ctx) => {
                    var _c;
                    (_c = this.ifBlocks[ctx]) === null || _c === void 0 ? void 0 : _c.childs.push(forBlockId);
                });
            });
            let oldItems = deepCopy(getDataArray());
            const renderForBlock = (async (items) => {
                await Promise.resolve();
                const containerElm = getNestedArrayValue(this.refMap, parentElementIndex);
                const insertionPointElm = getNestedArrayValue(this.refMap, refElementIndex);
                if (!(items != null && typeof items[Symbol.iterator] === "function")) {
                    throw new Error(`Items should be an iterable object`);
                }
                Array.from(items).forEach((item, index) => {
                    const fullIndices = [...parentIndices, index];
                    const lunasElm = renderItem(item, fullIndices);
                    const domElm = _createDomElementFromLunasElement(lunasElm);
                    setNestedArrayValue(this.refMap, [mapOffset, ...fullIndices], domElm);
                    containerElm.insertBefore(domElm, insertionPointElm);
                    afterRenderHook === null || afterRenderHook === void 0 ? void 0 : afterRenderHook(item, fullIndices);
                    if (fragmentFunc) {
                        const fragments = fragmentFunc(item, fullIndices);
                        createFragments(fragments, ifCtxUnderFor, forBlockId);
                    }
                    if (forCtx.length > 0) {
                        const lastFor = forCtx[forCtx.length - 1];
                        const lastForWithIndices = indices.slice(0, -1).length
                            ? `${lastFor}-${indices.slice(0, -1)}`
                            : lastFor;
                        this.forBlocks[lastForWithIndices].childs.push(forBlockId);
                    }
                });
                oldItems = deepCopy(getDataArray());
            }).bind(this);
            const toBeRendered = () => {
                console.log("prevIfCtx", prevIfCtx);
                console.log("Object.keys(this.ifBlockStates)", this.ifBlockStates);
                !prevIfCtx ||
                    console.log(`[prevIfCtx].every(
            (ctx) => this.ifBlockStates
          )`, [prevIfCtx].every((ctx) => this.ifBlockStates[indices ? `${ctx}-${indices}` : ctx]));
                return (!prevIfCtx ||
                    [prevIfCtx].every((ctx) => this.ifBlockStates[indices ? `${ctx}-${indices}` : ctx]));
            };
            this.forBlocks[forBlockId] = {
                cleanUp: [],
                childs: [],
                renderer: () => renderForBlock(getDataArray()),
            };
            const updateFunc = (() => {
                if (!toBeRendered()) {
                    return;
                }
                if (bitAnd(this.valUpdateMap, updateFlag)) {
                    const newItems = Array.from(getDataArray());
                    if (diffDetected(oldItems, newItems)) {
                        oldItems.forEach((_item, i) => {
                            const rs = resetMap(this.refMap, [mapOffset, ...parentIndices, i], mapLength);
                            for (const r of rs) {
                                if (r instanceof HTMLElement) {
                                    r.remove();
                                }
                            }
                        });
                        // ここで
                        if (this.forBlocks[forBlockId]) {
                            const { cleanUp, childs } = this.forBlocks[forBlockId];
                            cleanUp.forEach((f) => f());
                            this.forBlocks[forBlockId].cleanUp = [];
                            childs.forEach((child) => {
                                if (this.forBlocks[child]) {
                                    this.forBlocks[child].cleanUp.forEach((f) => f());
                                    this.forBlocks[child].cleanUp = [];
                                }
                            });
                        }
                        renderForBlock(newItems);
                    }
                }
            }).bind(this);
            if (!this.updateBlockFuncs.find((blk) => blk.name === forBlockId)) {
                this.updateBlockFuncs.push({
                    name: forBlockId,
                    type: BlockType.FOR,
                    updateFuncs: [],
                });
            }
            const forBlock = this.updateBlockFuncs.find((blk) => blk.name === forBlockId);
            if (forBlock) {
                forBlock.updateFuncs.push(updateFunc);
            }
            const latestForName = forCtx[forCtx.length - 1];
            if (latestForName) {
                const cleanUpFunc = (() => {
                    this.updateBlockFuncs.find((blk) => blk.name === forBlockId).updateFuncs = [];
                    const newIndices = copyAndPopArray(indices);
                    const latestForNameWithIndices = newIndices.length > 0
                        ? `${latestForName}-${newIndices}`
                        : latestForName;
                    const childs = this.forBlocks[latestForNameWithIndices].childs;
                    childs.forEach((child) => {
                        if (this.forBlocks[child]) {
                            this.forBlocks[child].cleanUp.forEach((f) => f());
                            this.updateBlockFuncs.find((blk) => blk.name === forBlockId).updateFuncs = [];
                            this.forBlocks[child].cleanUp = [];
                            this.forBlocks[child].childs = [];
                        }
                    });
                }).bind(this);
                const popedIndices = copyAndPopArray(indices);
                const latestForNameWithIndices = popedIndices.length > 0
                    ? `${latestForName}-${popedIndices}`
                    : latestForName;
                this.forBlocks[latestForNameWithIndices].cleanUp.push(cleanUpFunc);
            }
            if (!toBeRendered()) {
                return;
            }
            renderForBlock(getDataArray());
        }
    }.bind(this);
    const insertTextNodes = function (args, _assignmentLocation = 0) {
        const assignmentLocation = typeof _assignmentLocation === "number"
            ? [_assignmentLocation]
            : _assignmentLocation;
        for (const [amount, parentIdx, anchorIdx, text] of args) {
            for (let i = 0; i < amount; i++) {
                const txtNode = document.createTextNode(text !== null && text !== void 0 ? text : " ");
                const parentElm = getNestedArrayValue(this.refMap, parentIdx);
                const anchorElm = getNestedArrayValue(this.refMap, anchorIdx);
                parentElm.insertBefore(txtNode, anchorElm);
                setNestedArrayValue(this.refMap, assignmentLocation, txtNode);
                assignmentLocation[0]++;
            }
        }
    }.bind(this);
    const createFragments = function (fragments, ifCtx, latestForName) {
        for (const [[textContent, attributeName, defaultValue], _nodeIdx, depBit, fragmentType,] of fragments) {
            const nodeIdx = typeof _nodeIdx === "number" ? [_nodeIdx] : _nodeIdx;
            const fragmentUpdateFunc = (() => {
                if (ifCtx === null || ifCtx === void 0 ? void 0 : ifCtx.length) {
                    const blockRendered = ifCtx.every((ctxName) => this.ifBlockStates[ctxName]);
                    const blockAlreadyUpdated = ifCtx.every((ctxName) => this.blkUpdateMap[ctxName]);
                    if (!blockRendered || blockAlreadyUpdated) {
                        return;
                    }
                }
                const valueUpdated = bitAnd(this.valUpdateMap, depBit);
                if (!valueUpdated) {
                    return;
                }
                const target = getNestedArrayValue(this.refMap, nodeIdx);
                if (fragmentType === FragmentType.ATTRIBUTE) {
                    $$lunasReplaceAttr(attributeName, textContent(), defaultValue, target);
                }
                else {
                    $$lunasReplaceText(textContent(), target);
                }
            }).bind(this);
            if (fragmentType === FragmentType.ATTRIBUTE) {
                // Because the determination of the arribute types depends on dynamic values,
                // it is necessary to update the attributes after the initial rendering
                const target = getNestedArrayValue(this.refMap, nodeIdx);
                $$lunasReplaceAttr(attributeName, textContent(), defaultValue, target);
            }
            this.updateComponentFuncs[1].push(fragmentUpdateFunc);
            if (latestForName) {
                const cleanUpFunc = (() => {
                    const idx = this.updateComponentFuncs[1].indexOf(fragmentUpdateFunc);
                    this.updateComponentFuncs[1].splice(idx, 1);
                }).bind(this);
                this.forBlocks[latestForName].cleanUp.push(cleanUpFunc);
            }
        }
    }.bind(this);
    const lunasInsertComponent = function (componentExport, parentIdx, anchorIdx, refIdx, latestCtx, indices) {
        const parentElement = getNestedArrayValue(this.refMap, parentIdx);
        const anchorElement = getNestedArrayValue(this.refMap, anchorIdx);
        const { componentElm } = componentExport.insert(parentElement, anchorElement);
        setNestedArrayValue(this.refMap, refIdx, componentElm);
        if (latestCtx) {
            const forIndices = indices ? indices.slice(0, -1) : null;
            const forBlockName = (forIndices === null || forIndices === void 0 ? void 0 : forIndices.length)
                ? `${latestCtx}-${forIndices}`
                : latestCtx;
            const ifBlockName = indices ? `${latestCtx}-${indices}` : latestCtx;
            if (this.forBlocks[forBlockName]) {
                this.forBlocks[forBlockName].cleanUp.push(() => {
                    componentExport.__unmount();
                });
            }
            else if (this.ifBlocks[ifBlockName]) {
                this.ifBlocks[ifBlockName].cleanup.push(() => {
                    componentExport.__unmount();
                });
            }
        }
    }.bind(this);
    const lunasMountComponent = function (componentExport, parentIdx, refIdx, latestCtx, indices) {
        const parentElement = getNestedArrayValue(this.refMap, parentIdx);
        const { componentElm } = componentExport.mount(parentElement);
        setNestedArrayValue(this.refMap, refIdx, componentElm);
        if (latestCtx) {
            const forIndices = indices ? indices.slice(0, -1) : null;
            const forBlockName = (forIndices === null || forIndices === void 0 ? void 0 : forIndices.length)
                ? `${latestCtx}-${forIndices}`
                : latestCtx;
            const ifBlockName = indices ? `${latestCtx}-${indices}` : latestCtx;
            if (this.forBlocks[forBlockName]) {
                this.forBlocks[forBlockName].cleanUp.push(() => {
                    componentExport.__unmount();
                });
            }
            else if (this.ifBlocks[ifBlockName]) {
                this.ifBlocks[ifBlockName].cleanup.push(() => {
                    componentExport.__unmount();
                });
            }
        }
    }.bind(this);
    const watch = function (dependingVars, func) {
        // Create a combined dependency bit
        const combinedBits = [0];
        for (const depVar of dependingVars) {
            if (depVar instanceof valueObj) {
                const bit = this.currentVarBitGen.next().value;
                bitOrAssign(combinedBits, bit);
                depVar.addToCurrentDependency(this, bit);
            }
        }
        // Add an update function that calls func when any dependency changes
        const updateFunc = (() => {
            if (bitAnd(this.valUpdateMap, combinedBits)) {
                func();
            }
        }).bind(this);
        this.updateComponentFuncs[0].push(updateFunc);
    }.bind(this);
    return {
        $$lunasGetElm: getElm,
        $$lunasSetImportVars: setImportVars,
        $$lunasSetComponentElement: componentElementSetter,
        $$lunasApplyEnhancement: applyEnhancement,
        $$lunasAfterMount: setAfterMount,
        $$lunasAfterUnmount: setAfterUnmount,
        $$lunasReactive: createReactive,
        $$lunasCreateIfBlock: createIfBlock,
        $$lunasCreateForBlock: createForBlock,
        $$lunasRenderIfBlock: renderIfBlock,
        $$lunasGetElmRefs: getElmRefs,
        $$lunasInsertTextNodes: insertTextNodes,
        $$lunasAddEvListener: addEvListener,
        $$lunasCreateFragments: createFragments,
        $$lunasInsertComponent: lunasInsertComponent,
        $$lunasMountComponent: lunasMountComponent,
        $$lunasWatch: watch,
        $$lunasComponentReturn: {
            mount,
            insert,
            __unmount,
        },
    };
};
exports.$$lunasInitComponent = $$lunasInitComponent;
function $$lunasEscapeHtml(text) {
    const map = {
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#039;",
    };
    return String(text).replace(/[&<>"']/g, (m) => {
        return map[m];
    });
}
exports.$$lunasEscapeHtml = $$lunasEscapeHtml;
function $$lunasReplaceText(content, elm) {
    elm.textContent = $$lunasEscapeHtml(content);
}
exports.$$lunasReplaceText = $$lunasReplaceText;
// export function $$lunasReplaceInnerHtml(content: any, elm: HTMLElement) {
//   elm.innerHTML = $$lunasEscapeHtml(content);
// }
function $$lunasReplaceAttr(key, content, defaultValue, elm) {
    if (typeof content === "boolean") {
        if (content) {
            elm.setAttribute(key, "");
        }
        else if (elm.hasAttribute(key)) {
            elm.removeAttribute(key);
        }
        return;
    }
    else if (typeof content === "object") {
        let attrVal = defaultValue ? `${defaultValue} ` : "";
        attrVal += Object.keys(content)
            .filter((k) => content[k])
            .join(" ");
        elm.setAttribute(key, attrVal);
    }
    else {
        if (content === undefined && elm.hasAttribute(key)) {
            elm.removeAttribute(key);
            return;
        }
        elm[key] = String(content);
    }
}
exports.$$lunasReplaceAttr = $$lunasReplaceAttr;
function $$createLunasElement(innerHtml, topElmTag, topElmAttr = {}) {
    return {
        innerHtml,
        topElmTag,
        topElmAttr,
    };
}
exports.$$createLunasElement = $$createLunasElement;
const _createDomElementFromLunasElement = (lunasElement) => {
    const componentElm = document.createElement(lunasElement.topElmTag);
    Object.keys(lunasElement.topElmAttr).forEach((key) => {
        componentElm.setAttribute(key, lunasElement.topElmAttr[key]);
    });
    componentElm.innerHTML = lunasElement.innerHtml;
    return componentElm;
};
const $$lunasCreateNonReactive = function (v) {
    return new valueObj(v);
};
exports.$$lunasCreateNonReactive = $$lunasCreateNonReactive;
var FragmentType;
(function (FragmentType) {
    FragmentType[FragmentType["ATTRIBUTE"] = 0] = "ATTRIBUTE";
    FragmentType[FragmentType["TEXT"] = 1] = "TEXT";
    FragmentType[FragmentType["ELEMENT"] = 2] = "ELEMENT";
})(FragmentType || (FragmentType = {}));
function diffDetected(_oldArray, _newArray) {
    // return (
    //   oldArray.length !== newArray.length ||
    //   oldArray.some((v, i) => v !== newArray[i])
    // );
    // FIXME: This is a temporary implementation
    return true;
}
function setNestedArrayValue(arr, location, value) {
    const path = numberOrNumberArrayToNumberArray(location);
    let current = arr;
    for (let i = 0; i < path.length - 1; i++) {
        const key = path[i];
        if (current[key] === undefined) {
            current[key] = [];
        }
        current = current[key];
    }
    current[path[path.length - 1]] = value;
}
function getNestedArrayValue(arr, location) {
    if (location == null)
        return null;
    const path = numberOrNumberArrayToNumberArray(location);
    let current = arr;
    for (const key of path) {
        if (!Array.isArray(current) || current[key] == null) {
            return null;
        }
        current = current[key];
    }
    return current;
}
function numberOrNumberArrayToNumberArray(location) {
    return typeof location === "number" ? [location] : location;
}
function addNumberToArrayInitial(arr, num) {
    if (typeof arr === "number") {
        return [arr + num];
    }
    else {
        const copy = [...arr];
        copy[0] += num;
        return copy;
    }
}
function bitMapToBoolArr(bitMap) {
    if (typeof bitMap === "number") {
        return Array.from({ length: 31 }, (_, i) => (bitMap & (1 << i)) !== 0);
    }
    else {
        return bitMap
            .map((v) => bitMapToBoolArr(v))
            .reduce((acc, val) => acc.concat(val), []);
    }
}
// A function to perform bitwise "&" operation on number[] and number[]
function bitAnd(_a, _b) {
    const length = Math.max(typeof _a === "number" ? 1 : _a.length, typeof _b === "number" ? 1 : _b.length);
    const a = fillArrayWithZero(_a, length);
    const b = fillArrayWithZero(_b, length);
    return a.reduce((acc, val, i) => {
        return acc || (val & b[i]) !== 0;
    }, false);
}
function bitCombine(_a, _b) {
    const length = Math.max(typeof _a === "number" ? 1 : _a.length, typeof _b === "number" ? 1 : _b.length);
    const a = fillArrayWithZero(_a, length);
    const b = fillArrayWithZero(_b, length);
    const result = new Array(length);
    for (let i = 0; i < length; i++) {
        result[i] = a[i] | b[i];
    }
    return result;
}
// A function to perform bitwise "|=" operation on number[] and number[]
function bitOrAssign(target, source) {
    const length = Math.max(typeof target === "number" ? 1 : target.length, typeof source === "number" ? 1 : source.length);
    const targetArr = fillArrayWithZero(target, length);
    const sourceArr = fillArrayWithZero(source, length);
    for (let i = 0; i < length; i++) {
        targetArr[i] |= sourceArr[i];
    }
    if (typeof target === "number") {
        target = targetArr[0];
    }
    else {
        for (let i = 0; i < length; i++) {
            target[i] = targetArr[i];
        }
    }
}
// If the lengths of the arrays do not match, add 0 to the shorter array to match the length
function fillArrayWithZero(arr, length) {
    const array = typeof arr === "number" ? [arr] : arr;
    while (array.length < length) {
        array.push(0);
    }
    return array;
}
function resetMap(arr, mapLocation, length) {
    const results = [];
    let copied = deepCopy(mapLocation); // deep copy the mapLocation
    for (let i = 0; i < length; i++) {
        let target = arr;
        for (let i = 0; i < copied.length - 1; i++) {
            target = target[copied[i]];
        }
        const lastIndex = copied[copied.length - 1];
        const result = target[lastIndex];
        results.push(result);
        target[lastIndex] = undefined;
        copied = addNumberToArrayInitial(copied, 1);
    }
    return results;
}
function deepCopy(data) {
    if (data != null && typeof data === "object") {
        // Check if data is an iterator (has a next method)
        if (typeof data.next === "function") {
            return deepCopy(Array.from(data));
        }
        else if (Array.isArray(data)) {
            return data.map((item) => deepCopy(item));
        }
        else {
            const result = {};
            for (const key in data) {
                if (Object.prototype.hasOwnProperty.call(data, key)) {
                    result[key] = deepCopy(data[key]);
                }
            }
            return result;
        }
    }
    return data;
}
function* bitArrayGenerator() {
    const bitWidth = 31;
    let exp = 0;
    while (true) {
        const digitIndex = Math.floor(exp / bitWidth);
        const bitIndex = exp % bitWidth;
        const out = new Array(digitIndex + 1).fill(0);
        out[digitIndex] = 1 << bitIndex;
        yield out;
        exp++;
    }
}
function copyAndPopArray(arr) {
    const copy = arr.slice();
    copy.pop();
    return copy;
}
function isReactive(value) {
    return (typeof value === "object" &&
        value !== null &&
        "addDependency" in value &&
        typeof value.addDependency === "function" &&
        "addToCurrentDependency" in value &&
        typeof value.addToCurrentDependency === "function");
}
function reactive(initial, componentObj, componentSymbol, symbolIndex = [0]) {
    // 1) Create a valueObj instance that wraps the initial value.
    const wrapper = new valueObj(initial, componentObj, componentSymbol, symbolIndex);
    // 2) Get the generated Proxy (or primitive) reference.
    const proxy = wrapper.v;
    // 3) Directly attach the addDependency method to the Proxy object.
    Object.defineProperty(proxy, "addDependency", {
        value: (cObj, sIndex) => {
            return wrapper.addDependency(cObj, sIndex);
        },
        enumerable: false,
        writable: false,
        configurable: false,
    });
    // 4) Likewise, add addToCurrentDependency if needed.
    Object.defineProperty(proxy, "addToCurrentDependency", {
        value: (cObj, sIndex) => {
            wrapper.addToCurrentDependency(cObj, sIndex);
        },
        enumerable: false,
        writable: false,
        configurable: false,
    });
    return proxy;
}
exports.reactive = reactive;
