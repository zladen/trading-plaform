import { cva, type VariantProps } from "class-variance-authority";
import type { HTMLAttributes } from "react";

import { cn } from "@/shared/lib";

const badgeVariants = cva(
	"inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium whitespace-nowrap",
	{
		variants: {
			tone: {
				neutral: "border-slate-700 bg-slate-800/70 text-slate-300",
				positive: "border-emerald-500/40 bg-emerald-500/10 text-emerald-300",
				negative: "border-rose-500/40 bg-rose-500/10 text-rose-300",
				warning: "border-amber-500/40 bg-amber-500/10 text-amber-300",
				accent: "border-sky-500/40 bg-sky-500/10 text-sky-300",
			},
		},
		defaultVariants: {
			tone: "neutral",
		},
	},
);

export type BadgeProps = HTMLAttributes<HTMLSpanElement> & VariantProps<typeof badgeVariants>;

/** Compact status label. */
export function Badge({ className, tone, ...props }: BadgeProps) {
	return <span className={cn(badgeVariants({ tone }), className)} {...props} />;
}
