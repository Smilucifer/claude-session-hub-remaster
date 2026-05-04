export function routeMatches(currentPath: string, routePath: string): boolean {
  const normalizedRoute = routePath.endsWith("/") ? routePath.slice(0, -1) : routePath;
  return currentPath === normalizedRoute || currentPath.startsWith(`${normalizedRoute}/`);
}
