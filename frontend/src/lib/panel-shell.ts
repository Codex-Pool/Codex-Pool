export type PanelShellTone = 'primary' | 'secondary'

const PANEL_SHELL_CLASS_NAMES: Record<PanelShellTone, string> = {
  primary: 'border-small border-default-200 bg-content1 shadow-small',
  secondary: 'border-small border-default-200 bg-content2 shadow-none',
}

export function resolvePanelShellClassName(tone: PanelShellTone = 'primary') {
  return PANEL_SHELL_CLASS_NAMES[tone]
}
