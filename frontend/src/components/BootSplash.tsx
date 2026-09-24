import { useEffect, useState } from 'react';
import { useThemeStore } from '../stores/themeStore';

/**
 * APP 启动动画：
 * 阶段1（0~0.6s）：图标居中，弹性缩放进入
 * 阶段2（0.6~1.4s）：图标左移 + 名称从右侧滑入（非线性缓出）
 * 阶段3（1.4~1.9s）：整层淡出后调用 onDone 卸载
 */
function BootSplash({ onDone }: { onDone: () => void }) {
  const { mode } = useThemeStore();
  const [hidden, setHidden] = useState(false);

  // 阶段3完成后通知父组件卸载
  useEffect(() => {
    const t = setTimeout(() => {
      setHidden(true);
      onDone();
    }, 1900);
    return () => clearTimeout(t);
  }, [onDone]);

  if (hidden) return null;

  return (
    <div
      className="boot-splash-overlay"
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 9999,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: mode === 'dark' ? '#0a0a0c' : '#f5f5f7',
        animation: 'splashFadeOut 0.5s 1.4s var(--ease-out-expo) forwards',
      }}
    >
      <div
        className="boot-splash-content"
        style={{
          display: 'flex',
          alignItems: 'center',
          // 阶段2：整体左移，--splash-shift 控制偏移量
          animation: 'splashIconLeft 0.8s 0.6s var(--ease-out-expo) forwards',
          ['--splash-shift' as string]: '-120px',
        }}
      >
        <img
          src="/icons/icon.png"
          alt="BootTracker"
          width={56}
          height={56}
          style={{
            // 阶段1：弹性缩放进入
            animation: 'splashIconIn 0.6s var(--ease-spring) forwards',
          }}
        />
        <span
          className="boot-splash-name"
          style={{
            marginLeft: 14,
            fontSize: 26,
            fontWeight: 700,
            letterSpacing: 0.5,
            color: mode === 'dark' ? '#fafafa' : '#1e293b',
            // 阶段2：名称滑入
            opacity: 0,
            animation: 'splashNameIn 0.8s 0.6s var(--ease-out-expo) forwards',
          }}
        >
          BootTracker
        </span>
      </div>
    </div>
  );
}

export default BootSplash;
