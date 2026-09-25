import { useEffect } from 'react';
import { useThemeStore } from '../stores/themeStore';

/** 系统「减少动态效果」偏好：跳过所有过渡，仅快速淡出 */
const REDUCED_MOTION =
  typeof matchMedia !== 'undefined' &&
  matchMedia('(prefers-reduced-motion: reduce)').matches;

/**
 * APP 启动动画（紧凑节奏，约 1.35s）：
 * 阶段1（0~0.55s）：图标弹性缩放进入 + accent 光晕（跟随当前主题色）
 * 阶段2（0.3~0.9s）：名称 tracking-in（模糊 + 字距收敛），accent 进度线扫过
 * 阶段3（0.95~1.35s）：整层淡出后调用 onDone 卸载
 * 减少动态模式下：无位移无弹性，0.4s 直接淡出
 */
function BootSplash({ onDone }: { onDone: () => void }) {
  const { mode } = useThemeStore();

  useEffect(() => {
    const t = setTimeout(onDone, REDUCED_MOTION ? 400 : 1350);
    return () => clearTimeout(t);
  }, [onDone]);

  return (
    <div
      className="boot-splash-overlay"
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 9999,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        // 不透明纯色：Mica 主题下 --bg-page 为半透明，splash 需完全遮住底层 UI
        background: mode === 'dark' ? '#0a0a0c' : '#f8fafc',
        animation: REDUCED_MOTION
          ? 'splashFadeOut 0.3s 0.1s forwards'
          : 'splashFadeOut 0.4s 0.95s var(--ease-out-expo) forwards',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center' }}>
        <img
          className="boot-splash-icon"
          src="/icons/icon.png"
          alt="BootTracker"
          width={56}
          height={56}
          style={{
            animation: 'splashIconIn 0.55s var(--ease-spring) forwards',
          }}
        />
        <span
          className="boot-splash-name"
          style={{
            marginLeft: 14,
            fontSize: 26,
            fontWeight: 700,
            color: mode === 'dark' ? '#fafafa' : '#1e293b',
            opacity: 0,
            animation: 'splashNameIn 0.6s 0.3s var(--ease-out-expo) forwards',
          }}
        >
          BootTracker
        </span>
      </div>

      {/* accent 进度线：品牌色扫过，暗示加载进程 */}
      <div
        className="boot-splash-progress"
        style={{
          marginTop: 22,
          width: 132,
          height: 2,
          borderRadius: 1,
          overflow: 'hidden',
          background: mode === 'dark' ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.06)',
        }}
      >
        <div
          className="boot-splash-progress-bar"
          style={{
            height: '100%',
            borderRadius: 1,
            background: 'var(--accent)',
            transformOrigin: 'left',
            animation: REDUCED_MOTION
              ? 'splashProgress 0.3s linear forwards'
              : 'splashProgress 1.25s var(--ease-out-expo) forwards',
          }}
        />
      </div>
    </div>
  );
}

export default BootSplash;
