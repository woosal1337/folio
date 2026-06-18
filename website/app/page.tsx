import { Hero } from "@/components/landing/hero";
import { TrustBar } from "@/components/landing/trust-bar";
import { FeatureGrid } from "@/components/landing/feature-grid";
import { HowItWorks } from "@/components/landing/how-it-works";
import { PrivacySection } from "@/components/landing/privacy-section";
import { ConnectorsSection } from "@/components/landing/connectors-section";
import { InstallSection } from "@/components/landing/install-section";
import { FaqSection } from "@/components/landing/faq-section";
import { CtaSection } from "@/components/landing/cta-section";

export default function HomePage() {
  return (
    <>
      <Hero />
      <TrustBar />
      <FeatureGrid />
      <HowItWorks />
      <PrivacySection />
      <ConnectorsSection />
      <InstallSection />
      <FaqSection />
      <CtaSection />
    </>
  );
}
