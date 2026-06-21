"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.$$lunasRouter = exports.Router = void 0;
class Router {
    constructor() {
        this.routes = [];
        this.notFound = () => { };
        this.currentComponent = null;
        window.addEventListener("popstate", this.handlePopState.bind(this));
    }
    addRoute(path, componentLoader) {
        this.routes.push({ path, component: componentLoader });
    }
    setNotFound(notFoundHandler) {
        this.notFound = notFoundHandler;
    }
    navigate(path) {
        window.history.pushState({}, path, window.location.origin + path);
        this.handleRoute(path);
    }
    async handleRoute(path) {
        const route = this.routes.find((route) => route.path === path);
        if (route) {
            const component = (await route.component()).default;
            this.renderComponent(component);
        }
        else {
            this.notFound();
        }
    }
    handlePopState() {
        this.handleRoute(window.location.pathname);
    }
    renderComponent(component) {
        if (this.currentComponent) {
            this.currentComponent.__unmount();
        }
        this.currentComponent = component();
        if (this.renderingTarget.haveSiblingElm) {
            this.currentComponent.insert(this.renderingTarget.parent, this.renderingTarget.anchor);
        }
        else {
            this.currentComponent.mount(this.renderingTarget.parent);
        }
    }
    initialize(routes = [], parent, anchor, haveSiblingElm) {
        this.routes = routes;
        this.renderingTarget = { parent, anchor, haveSiblingElm };
        this.handleRoute(window.location.pathname);
    }
}
exports.Router = Router;
exports.$$lunasRouter = new Router();
