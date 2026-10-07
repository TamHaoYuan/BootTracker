import type { ThemeConfig } from 'antd';
import { theme as antdTheme } from 'antd';

export type AppMode = 'dark' | 'light';
export type ThemeName =
  | 'purple'
  | 'blue'
  | 'green'
  | 'orange'
  | 'gray'
  | 'mica'
  | 'material-you';

// 各主题的强调色（与 tokens.css [data-theme="..."] 保持一致）
export const THEME_ACCENTS: Record<ThemeName, { primary: string; secondary: string }> = {
  purple: { primary: '#8b5cf6', secondary: '#d946ef' },
  blue: { primary: '#3b82f6', secondary: '#06b6d4' },
  green: { primary: '#10b981', secondary: '#34d399' },
  orange: { primary: '#f97316', secondary: '#f59e0b' },
  gray: { primary: '#64748b', secondary: '#94a3b8' },
  mica: { primary: '#fafafa', secondary: '#c4c4cc' },
  'material-you': { primary: '#d0bcff', secondary: '#b69df8' },
};

/** 根据模式与主题生成 Ant Design 配置 */
export function buildAntdTheme(mode: AppMode, theme: ThemeName): ThemeConfig {
  const accent = THEME_ACCENTS[theme];
  return {
    algorithm: mode === 'dark' ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
    token: {
      colorPrimary: accent.primary,
      colorInfo: accent.secondary,
      colorBgBase: mode === 'dark' ? '#0a0a0c' : '#f8fafc',
      // 注意：不要把 colorBgElevated / colorBgContainer 指向 CSS 变量——
      // AntD 会用 seed 派生灰阶，tinycolor 解析不了 `var(...)` 会退化成黑色，
      // 反而把浮层染灰。浮层底色统一由 styles/global.css 的弹层规则接管。
      colorBorderSecondary: 'var(--border-color)',
      colorTextBase: mode === 'dark' ? '#fafafa' : '#0f172a',
      colorBorder: mode === 'dark' ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.08)',
      borderRadius: 8,
      fontFamily: "'Inter', 'Noto Sans SC', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    },
    components: {
      Layout: {
        // 注：旧 token colorBgHeader / colorBgBody 在 AntD 5.x 已 deprecated
        headerBg: 'transparent',
        bodyBg: 'transparent',
      },
      Card: {
        colorBgContainer: 'var(--bg-card)',
        colorBorderSecondary: 'var(--border-color)',
      },
      Table: {
        colorBgContainer: 'transparent',
        headerBg: 'var(--bg-card-light)',
        headerColor: 'var(--text-secondary)',
        rowHoverBg: 'var(--glass-hover)',
      },
      Button: {
        controlHeight: 36,
      },
    },
  };
}
