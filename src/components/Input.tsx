import type { InputHTMLAttributes } from "react";

export function Input({ className, ...rest }: InputHTMLAttributes<HTMLInputElement>) {
  const classes = ["input", className ?? ""].filter(Boolean).join(" ");
  return <input className={classes} {...rest} />;
}