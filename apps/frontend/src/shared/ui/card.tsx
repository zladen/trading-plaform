import type { HTMLAttributes } from "react";

import { cn } from "@/shared/lib";

/** Surface container used for dashboard sections. */
export function Card({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
	return (
		<div
			className={cn(
				"rounded-xl border border-slate-800 bg-slate-900/60 shadow-sm backdrop-blur",
				className,
			)}
			{...props}
		/>
	);
}

/** Card heading region. */
export function CardHeader({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
	return <div className={cn("flex flex-col gap-1 p-5 pb-3", className)} {...props} />;
}

/** Card title. */
export function CardTitle({ className, ...props }: HTMLAttributes<HTMLHeadingElement>) {
	return (
		<h2
			className={cn(
				"text-sm font-semibold tracking-wide text-slate-200 uppercase",
				className,
			)}
			{...props}
		/>
	);
}

/** Card supporting text. */
export function CardDescription({ className, ...props }: HTMLAttributes<HTMLParagraphElement>) {
	return <p className={cn("text-sm text-slate-400", className)} {...props} />;
}

/** Card content region. */
export function CardContent({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
	return <div className={cn("p-5 pt-0", className)} {...props} />;
}
