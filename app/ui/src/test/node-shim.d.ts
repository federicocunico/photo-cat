// Tipi minimi dei moduli Node usati dai test che leggono i sorgenti dal disco (ad esempio i CSS,
// che Vitest con `css: false` restituisce vuoti). Il progetto non include @types/node per non
// mescolare i tipi del browser con quelli di Node nel codice dell'interfaccia.
declare module "node:fs" {
  interface Dirent {
    name: string;
    isDirectory(): boolean;
  }
  export function readdirSync(dir: string, options: { withFileTypes: true }): Dirent[];
  export function readFileSync(file: string, encoding: "utf8"): string;
  const fs: { readdirSync: typeof readdirSync; readFileSync: typeof readFileSync };
  export default fs;
}

declare module "node:path" {
  export function join(...parts: string[]): string;
  export function resolve(...parts: string[]): string;
  export function dirname(p: string): string;
  export function relative(from: string, to: string): string;
  const path: {
    join: typeof join;
    resolve: typeof resolve;
    dirname: typeof dirname;
    relative: typeof relative;
  };
  export default path;
}

declare module "node:url" {
  export function fileURLToPath(url: string): string;
}
