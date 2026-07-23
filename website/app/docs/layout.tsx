import { DocsSidebar } from "@/components/docs/docs-sidebar";
import { DocsMobileNav } from "@/components/docs/docs-mobile-nav";
import { DocPager } from "@/components/docs/doc-pager";

export default function DocsLayout({ children }: { children: React.ReactNode }) {
    return (
        <div className="container py-10 lg:py-14">
            <div className="lg:grid lg:grid-cols-[15rem_minmax(0,1fr)] lg:gap-12 xl:gap-16">
                <aside className="hidden lg:block">
                    <div className="sticky top-24">
                        <DocsSidebar />
                    </div>
                </aside>

                <div className="min-w-0">
                    <div className="mb-8 lg:hidden">
                        <DocsMobileNav />
                    </div>
                    <article className="max-w-prose">
                        {children}
                        <DocPager />
                    </article>
                </div>
            </div>
        </div>
    );
}
