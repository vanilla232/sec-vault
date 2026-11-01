import { useEffect, useRef, useState, type ReactNode } from "react";

interface Props {
  label: string;
  children: ReactNode;
}

export function Tooltip({ label, children }: Props) {
  const [visible, setVisible] = useState(false);
  const timerRef = useRef<number | null>(null);

  const show = () => {
    timerRef.current = window.setTimeout(() => setVisible(true), 400);
  };

  const hide = () => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setVisible(false);
  };

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    };
  }, []);

  return (
    <span className="tooltip-wrap" onMouseEnter={show} onMouseLeave={hide}>
      {children}
      <span className={`tooltip${visible ? " visible" : ""}`} role="tooltip">
        {label}
      </span>
    </span>
  );
}