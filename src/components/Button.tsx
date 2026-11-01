import type { ButtonHTMLAttributes } from "react";

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "primary" | "danger";
}

export function Button({ variant = "default", className, ...rest }: Props) {
  const classes = [variant !== "default" ? variant : "", className ?? ""]
    .filter(Boolean)
    .join(" ");
  return <button className={classes || undefined} {...rest} />;
}