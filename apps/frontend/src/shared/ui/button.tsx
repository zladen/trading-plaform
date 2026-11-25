import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import type { ButtonHTMLAttributes } from "react";

import { cn } from "@/shared/lib";

const buttonVariants = cva(
	"inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium transition-colors outline-none focus-visible:ring-2 focus-visible:ring-sky-400/60 focus-visible:ring-offset-2 focus-visible:ring-offset-slate-950 disabled:pointer-events-none disabled:opacity-50",
	{
		variants: {
			variant: {
				default: "bg-sky-500 text-slate-950 hover:bg-sky-400",
				outline:
					"border border-slate-700 bg-slate-900/60 text-slate-100 hover:border-slate-500 hover:bg-slate-800",
				ghost: "text-slate-300 hover:bg-slate-800 hover:text-slate-100",
			},
			size: {
				default: "h-9 px-4 py-2",
				sm: "h-8 rounded-md px-3 text-xs",
				icon: "size-9",
			},
		},
		defaultVariants: {
			variant: "default",
			size: "default",
		},
	},
);

export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> &
	VariantProps<typeof buttonVariants> & {
		asChild?: boolean;
	};

/**
 * Accessible button with the project's shadcn/ui style variants.
 *
 * `asChild` renders the styles onto the single child element, which is how
 * links are presented as buttons without nesting interactive elements.
 */
export function Button({ className, variant, size, asChild = false, ...props }: ButtonProps) {
	const Component = asChild ? Slot : "button";

	return <Component className={cn(buttonVariants({ variant, size }), className)} {...props} />;
}
