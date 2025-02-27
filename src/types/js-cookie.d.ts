declare module 'js-cookie' {
  export function get(name: string): string | undefined;
  export function set(name: string, value: string, options?: any): void;
  export function remove(name: string, options?: any): void;
}