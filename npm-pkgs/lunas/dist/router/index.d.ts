import type { ComponentDeclaration, LunasModuleExports } from "../engine";
export type ComponentLoader = () => Promise<{
    default: ComponentDeclaration;
}>;
export type Route = {
    path: string;
    component: ComponentLoader;
};
export declare class Router {
    routes: Route[];
    notFound: () => void;
    currentComponent: LunasModuleExports | null;
    renderingTarget: {
        parent: HTMLElement;
        anchor: HTMLElement | null;
        haveSiblingElm: boolean;
    };
    constructor();
    addRoute(path: string, componentLoader: ComponentLoader): void;
    setNotFound(notFoundHandler: () => void): void;
    navigate(path: string): void;
    handleRoute(path: string): Promise<void>;
    handlePopState(): void;
    renderComponent(component: ComponentDeclaration): void;
    initialize(routes: Route[] | undefined, parent: HTMLElement, anchor: HTMLElement | null, haveSiblingElm: boolean): void;
}
export declare const $$lunasRouter: Router;
