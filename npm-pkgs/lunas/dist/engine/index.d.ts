/** biome-ignore-all lint/suspicious/noExplicitAny: user inputs are unpredictable, so accepting `any` is necessary to handle arbitrary data. */
/** biome-ignore-all lint/suspicious/noPrototypeBuiltins: ensures compatibility with environments where Object.prototype methods may be shadowed or customized. */
/** biome-ignore-all lint/style/noNonNullAssertion: earlier validation guarantees non-null values, but TypeScript cannot infer this fact. */
export type ComponentDeclaration = (args?: {
    [key: string]: any;
}) => LunasModuleExports;
export type LunasModuleExports = {
    mount: (elm: HTMLElement) => LunasComponentState;
    insert: (elm: HTMLElement, anchor: HTMLElement | null) => LunasComponentState;
    __unmount: () => void;
};
declare enum BlockType {
    IF = "IF",
    FOR = "FOR"
}
export type UpdateBlockFuncs = {
    name: string;
    type: BlockType;
    updateFuncs: (() => void)[];
}[];
export type LunasComponentState = {
    updatedFlag: boolean;
    valUpdateMap: number[];
    internalElement: LunasInternalElement;
    currentVarBitGen: Generator<number[]>;
    ifBlocks: {
        [key: string]: {
            renderer: () => void;
            context: string[];
            forBlk: string | null;
            condition: () => boolean;
            cleanup: (() => void)[];
            childs: string[];
            nextForBlocks: string[];
        };
    };
    ifBlockStates: Record<string, boolean>;
    blkUpdateMap: Record<string, boolean>;
    updateComponentFuncs: ((() => void) | undefined)[][];
    updateBlockFuncs: UpdateBlockFuncs;
    isMounted: boolean;
    componentElm: HTMLElement;
    compSymbol: symbol;
    resetDependecies: (() => void)[];
    __lunas_update: (() => void) | undefined;
    __lunas_apply_enhancement: () => void;
    __lunas_after_mount: () => void;
    __lunas_destroy: () => void;
    forBlocks: {
        [key: string]: {
            cleanUp: (() => void)[];
            childs: string[];
            renderer: () => void;
        };
    };
    refMap: RefMap;
};
type LunasInternalElement = {
    innerHtml: string;
    topElmTag: string;
    topElmAttr: {
        [key: string]: string;
    };
};
type FragmentFunc = (item?: unknown, indices?: number[]) => Fragment[];
export declare class valueObj<T> {
    private _v;
    private proxy;
    dependencies: {
        [key: symbol]: [LunasComponentState, number[]];
    };
    constructor(initialValue: T, componentObj?: LunasComponentState, componentSymbol?: symbol, symbolIndex?: number[]);
    set v(v: T);
    get v(): T;
    private triggerUpdate;
    private createProxy;
    addDependency(componentObj: LunasComponentState, symbolIndex: number[]): {
        removeDependency: () => void;
    };
    addToCurrentDependency(componentObj: LunasComponentState, symbolIndex: number[]): void;
}
export declare const $$lunasInitComponent: (this: LunasComponentState, args?: {
    [key: string]: any;
}, inputs?: string[]) => {
    $$lunasGetElm: (location: number | number[]) => Node | null | undefined;
    $$lunasSetImportVars: (items: unknown[]) => void;
    $$lunasSetComponentElement: (innerHtml: string, topElmTag: string, topElmAttr?: {
        [key: string]: string;
    } | undefined) => void;
    $$lunasApplyEnhancement: (enhancementFunc: () => void) => void;
    $$lunasAfterMount: (afterMount: () => void) => void;
    $$lunasAfterUnmount: (afterUnmount: () => void) => void;
    $$lunasReactive: (v: unknown) => valueObj<unknown>;
    $$lunasCreateIfBlock: (ifBlocks: [forBlockId: string | (() => string), lunasElement: () => LunasInternalElement, condition: () => boolean, postRender: () => void, ifCtx: string[], forCtx: string[], depBit: number | number[], mapInfo: [mapOffset: number | number[], mapLength: number], refIdx: [parentElementIndex: number | number[], refElementIndex?: number | number[] | undefined], fragment?: Fragment[] | undefined][], indices?: number[] | undefined) => void;
    $$lunasCreateForBlock: (forBlocksConfig: [forBlockId: string | (() => string), renderItem: (item: unknown, indices: number[]) => LunasInternalElement, getDataArray: () => unknown[], afterRenderHook: (item: unknown, indices: number[]) => void, ifCtxUnderFor: string[], forCtx: string[], prevIfCtx: string | null, updateFlag: number | number[], parentIndices: number[], mapInfo: [mapOffset: number, mapLength: number], refIdx: [parentElementIndex: number | number[], refElementIndex?: number | number[] | undefined], fragment?: FragmentFunc | undefined][], indices?: number[] | undefined) => void;
    $$lunasRenderIfBlock: (name: string) => void;
    $$lunasGetElmRefs: (ids: string[], preserveId: number | number[], refLocation?: number | number[] | undefined) => void;
    $$lunasInsertTextNodes: (args: [amount: number, parent: number | number[], anchor?: number | number[] | undefined, text?: string | undefined][], _assignmentLocation?: number | number[] | undefined) => void;
    $$lunasAddEvListener: (args: [number | number[], string, EventListener][]) => void;
    $$lunasCreateFragments: (fragments: Fragment[], ifCtx?: string[] | undefined, latestForName?: string | undefined) => void;
    $$lunasInsertComponent: (componentExport: LunasModuleExports, parentIdx: number | number[], anchorIdx: number | number[] | null, refIdx: number | number[], latestCtx: string | null, indices: number[] | null) => void;
    $$lunasMountComponent: (componentExport: LunasModuleExports, parentIdx: number | number[], refIdx: number | number[], latestCtx: string | null, indices: number[] | null) => void;
    $$lunasWatch: (dependingVars: unknown[], func: () => void) => void;
    $$lunasComponentReturn: LunasModuleExports;
};
export declare function $$lunasEscapeHtml(text: any): string;
export declare function $$lunasReplaceText(content: any, elm: Node): void;
export declare function $$lunasReplaceAttr(key: string, content: any, defaultValue: string | undefined, elm: HTMLElement): void;
export declare function $$createLunasElement(innerHtml: string, topElmTag: string, topElmAttr?: {
    [key: string]: string;
}): LunasInternalElement;
export declare const $$lunasCreateNonReactive: <T>(this: LunasComponentState, v: T) => valueObj<T>;
type Fragment = [
    content: [
        textContent: () => string,
        attributeName?: string,
        defaultValue?: string
    ],
    nodeIdx: number[] | number,
    depBit: number | number[],
    fragmentType: FragmentType
];
type RefMapItem = Node | undefined | RefMapItem[];
type RefMap = RefMapItem[];
declare enum FragmentType {
    ATTRIBUTE = 0,
    TEXT = 1,
    ELEMENT = 2
}
export type ReactiveWrapper<T> = T & {
    addDependency: (componentObj: LunasComponentState, symbolIndex: number[]) => {
        removeDependency: () => void;
    };
    addToCurrentDependency: (componentObj: LunasComponentState, symbolIndex: number[]) => void;
};
export declare function reactive<T extends object>(initial: T, componentObj?: LunasComponentState, componentSymbol?: symbol, symbolIndex?: number[]): T;
export {};
