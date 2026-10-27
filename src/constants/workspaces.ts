export const DEFAULT_WORKSPACE_ID = "wsp_default";

export function workspaceLabel(workspace: { id: string; name: string }): string {
  if (workspace.id === DEFAULT_WORKSPACE_ID || workspace.name === "default") {
    return "Default";
  }
  return workspace.name;
}
