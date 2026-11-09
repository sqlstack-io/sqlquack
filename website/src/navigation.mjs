import sidebarData from './sidebar.json';

export const sidebar = sidebarData;

// Pages that moved, so older links still land: the schema file page became Projects.
export const redirects = { '/docs/schema': '/docs/project/' };
