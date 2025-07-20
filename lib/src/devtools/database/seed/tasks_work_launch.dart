import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/foundation.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/task/task.entity.dart';
import '../../../things/block/block.entity.dart';
import '../../../things/thing.entity.dart';
import '../seed.dart';
import '_seed.dart';

class Q1LaunchProject extends DatabaseSeed {
  final seedProject = ProjectEntity(
    title: AttributedText("Q1 Product Launch"),
    uuid: "ad8fd8d8-974c-49ff-bbfc-b474ccca3467",
  );

  // Project description blocks
  final seedProjectHeaderBlock = BlockEntity(
    uuid: "launch-proj-header-001",
    nodeType: "header1",
    content: AttributedText("Q1 2025 Product Launch"),
  );

  final seedProjectDetailsBlock = BlockEntity(
    uuid: "launch-proj-details-002",
    nodeType: "paragraph",
    content: AttributedText(
      "Launch Date: March 15, 2025 | Product: Tempo Analytics Pro - Advanced analytics dashboard with AI insights",
    ),
  );

  final seedStrategyHeaderBlock = BlockEntity(
    uuid: "launch-proj-strategy-003",
    nodeType: "header2",
    content: AttributedText("Launch Strategy"),
  );

  final seedStrategyBlock = BlockEntity(
    uuid: "launch-proj-strategy-004",
    nodeType: "paragraph",
    content: AttributedText(
      "Coordinated product and marketing launch targeting enterprise customers. Focus on demonstrating ROI through automated insights and custom reporting capabilities.",
    ),
  );

  final seedFeaturesHeaderBlock = BlockEntity(
    uuid: "launch-proj-features-005",
    nodeType: "header2",
    content: AttributedText("Key Features"),
  );

  final seedFeaturesListBlock1 = BlockEntity(
    uuid: "launch-proj-features-006",
    nodeType: "list_item",
    content: AttributedText(
      "Real-time Dashboard: Live metrics with customizable widgets",
    ),
  );

  final seedFeaturesListBlock2 = BlockEntity(
    uuid: "launch-proj-features-007",
    nodeType: "list_item",
    content: AttributedText(
      "AI Insights: Automated anomaly detection and recommendations",
    ),
  );

  final seedFeaturesListBlock3 = BlockEntity(
    uuid: "launch-proj-features-008",
    nodeType: "list_item",
    content: AttributedText(
      "Export Engine: Support for PDF, Excel, and API integrations",
    ),
  );

  final seedFeaturesListBlock4 = BlockEntity(
    uuid: "launch-proj-features-009",
    nodeType: "list_item",
    content: AttributedText(
      "Enterprise Auth: SSO, RBAC, and audit tempoDBSeedLog.ging",
    ),
  );

  final seedMetricsHeaderBlock = BlockEntity(
    uuid: "launch-proj-metrics-010",
    nodeType: "header2",
    content: AttributedText("Success Metrics"),
  );

  final seedMetricsListBlock1 = BlockEntity(
    uuid: "launch-proj-metrics-011",
    nodeType: "list_item",
    content: AttributedText("500 beta signups in first week"),
  );

  final seedMetricsListBlock2 = BlockEntity(
    uuid: "launch-proj-metrics-012",
    nodeType: "list_item",
    content: AttributedText("50 enterprise demos booked"),
  );

  final seedMetricsListBlock3 = BlockEntity(
    uuid: "launch-proj-metrics-013",
    nodeType: "list_item",
    content: AttributedText("15% conversion rate from trial to paid"),
  );

  final seedMetricsListBlock4 = BlockEntity(
    uuid: "launch-proj-metrics-014",
    nodeType: "list_item",
    content: AttributedText("Featured in 3 major industry publications"),
  );

  final seedFeatureDevTask = DevtoolsTaskDefaults.withDefaults(
    "906ed45c-c56d-45a6-b24c-f109f90155d9",
    AttributedText("Complete feature development"),
    status: TaskStatus.workInProgress,
    dateDelta: const Duration(days: -2),
    priority: TaskPriority.H,
  );

  // Feature development task description blocks
  final seedFeatureDevHeaderBlock = BlockEntity(
    uuid: "launch-featuredev-header-015",
    nodeType: "header3",
    content: AttributedText("Final development sprint for Tempo Analytics Pro"),
  );

  final seedFeatureDevStatusBlock = BlockEntity(
    uuid: "launch-featuredev-status-016",
    nodeType: "paragraph",
    content: AttributedText("Current Status: 75% complete, 2 days overdue"),
  );

  final seedFeatureDevWorkHeaderBlock = BlockEntity(
    uuid: "launch-featuredev-work-017",
    nodeType: "header4",
    content: AttributedText("Remaining Work"),
  );

  final seedFeatureDevWorkBlock1 = BlockEntity(
    uuid: "launch-featuredev-work-018",
    nodeType: "list_item",
    content: AttributedText("Finalize AI insights algorithm (ML team)"),
  );

  final seedFeatureDevWorkBlock2 = BlockEntity(
    uuid: "launch-featuredev-work-019",
    nodeType: "list_item",
    content: AttributedText("Complete enterprise authentication integration"),
  );

  final seedFeatureDevWorkBlock3 = BlockEntity(
    uuid: "launch-featuredev-work-020",
    nodeType: "list_item",
    content: AttributedText(
      "Polish dashboard performance for 1000+ concurrent users",
    ),
  );

  final seedFeatureDevBlockersBlock = BlockEntity(
    uuid: "launch-featuredev-blockers-021",
    nodeType: "paragraph",
    content: AttributedText(
      "Blockers: AWS infrastructure scaling approval pending, security audit findings need resolution.",
    ),
  );

  final seedFeatureDevMeetingsBlock = BlockEntity(
    uuid: "launch-featuredev-meetings-022",
    nodeType: "paragraph",
    content: AttributedText(
      "Daily standups at 9:00 AM, all-hands demo Friday.",
    ),
  );

  final seedUserAuthTask = DevtoolsTaskDefaults.withDefaults(
    "265a8582-aba3-48a5-97c1-964a8d749db7",
    AttributedText("Implement user authentication flow"),
    status: TaskStatus.complete,
  );

  // User auth task description blocks
  final seedUserAuthHeaderBlock = BlockEntity(
    uuid: "launch-userauth-header-023",
    nodeType: "header3",
    content: AttributedText("Enterprise-grade authentication system ✅"),
  );

  final seedUserAuthFeaturesHeaderBlock = BlockEntity(
    uuid: "launch-userauth-features-024",
    nodeType: "header4",
    content: AttributedText("Implemented Features"),
  );

  final seedUserAuthFeaturesBlock1 = BlockEntity(
    uuid: "launch-userauth-features-025",
    nodeType: "list_item",
    content: AttributedText("SAML 2.0 and OIDC support"),
  );

  final seedUserAuthFeaturesBlock2 = BlockEntity(
    uuid: "launch-userauth-features-026",
    nodeType: "list_item",
    content: AttributedText("Role-based access control (RBAC)"),
  );

  final seedUserAuthFeaturesBlock3 = BlockEntity(
    uuid: "launch-userauth-features-027",
    nodeType: "list_item",
    content: AttributedText("Multi-factor authentication"),
  );

  final seedUserAuthFeaturesBlock4 = BlockEntity(
    uuid: "launch-userauth-features-028",
    nodeType: "list_item",
    content: AttributedText("Session management with Redis"),
  );

  final seedUserAuthFeaturesBlock5 = BlockEntity(
    uuid: "launch-userauth-features-029",
    nodeType: "list_item",
    content: AttributedText("Audit tempoDBSeedLog.ging for compliance"),
  );

  final seedUserAuthSecurityHeaderBlock = BlockEntity(
    uuid: "launch-userauth-security-030",
    nodeType: "header4",
    content: AttributedText("Security Features"),
  );

  final seedUserAuthSecurityBlock1 = BlockEntity(
    uuid: "launch-userauth-security-031",
    nodeType: "list_item",
    content: AttributedText("JWT tokens with 15-minute expiry"),
  );

  final seedUserAuthSecurityBlock2 = BlockEntity(
    uuid: "launch-userauth-security-032",
    nodeType: "list_item",
    content: AttributedText("Refresh token rotation"),
  );

  final seedUserAuthSecurityBlock3 = BlockEntity(
    uuid: "launch-userauth-security-033",
    nodeType: "list_item",
    content: AttributedText("Rate limiting on auth endpoints"),
  );

  final seedUserAuthSecurityBlock4 = BlockEntity(
    uuid: "launch-userauth-security-034",
    nodeType: "list_item",
    content: AttributedText("Password policy enforcement"),
  );

  final seedUserAuthTestingBlock = BlockEntity(
    uuid: "launch-userauth-testing-035",
    nodeType: "paragraph",
    content: AttributedText(
      "Testing: Passed penetration testing and SOC 2 Type II requirements.",
    ),
  );

  final seedAnalyticsDashboardTask = DevtoolsTaskDefaults.withDefaults(
    "4110eaee-405d-452f-b5bd-9ceac2af094f",
    AttributedText("Build dashboard analytics view"),
  );

  // Analytics dashboard task description blocks
  final seedAnalyticsHeaderBlock = BlockEntity(
    uuid: "launch-analytics-header-036",
    nodeType: "header3",
    content: AttributedText("Interactive Analytics Dashboard"),
  );

  final seedAnalyticsFeaturesHeaderBlock = BlockEntity(
    uuid: "launch-analytics-features-037",
    nodeType: "header4",
    content: AttributedText("Core Features"),
  );

  final seedAnalyticsFeaturesBlock1 = BlockEntity(
    uuid: "launch-analytics-features-038",
    nodeType: "list_item",
    content: AttributedText("Drag-and-drop widget customization"),
  );

  final seedAnalyticsFeaturesBlock2 = BlockEntity(
    uuid: "launch-analytics-features-039",
    nodeType: "list_item",
    content: AttributedText("Real-time data streaming with WebSockets"),
  );

  final seedAnalyticsFeaturesBlock3 = BlockEntity(
    uuid: "launch-analytics-features-040",
    nodeType: "list_item",
    content: AttributedText("15+ chart types (line, bar, heatmap, funnel)"),
  );

  final seedAnalyticsFeaturesBlock4 = BlockEntity(
    uuid: "launch-analytics-features-041",
    nodeType: "list_item",
    content: AttributedText("Custom date range filtering"),
  );

  final seedAnalyticsFeaturesBlock5 = BlockEntity(
    uuid: "launch-analytics-features-042",
    nodeType: "list_item",
    content: AttributedText("Export to PNG/PDF with branded templates"),
  );

  final seedAnalyticsTechHeaderBlock = BlockEntity(
    uuid: "launch-analytics-tech-043",
    nodeType: "header4",
    content: AttributedText("Technical Stack"),
  );

  final seedAnalyticsTechBlock1 = BlockEntity(
    uuid: "launch-analytics-tech-044",
    nodeType: "list_item",
    content: AttributedText("React + TypeScript frontend"),
  );

  final seedAnalyticsTechBlock2 = BlockEntity(
    uuid: "launch-analytics-tech-045",
    nodeType: "list_item",
    content: AttributedText("D3.js for custom visualizations"),
  );

  final seedAnalyticsTechBlock3 = BlockEntity(
    uuid: "launch-analytics-tech-046",
    nodeType: "list_item",
    content: AttributedText("Apache Kafka for real-time streams"),
  );

  final seedAnalyticsTechBlock4 = BlockEntity(
    uuid: "launch-analytics-tech-047",
    nodeType: "list_item",
    content: AttributedText("Redis for caching frequently accessed data"),
  );

  final seedAnalyticsPerfHeaderBlock = BlockEntity(
    uuid: "launch-analytics-perf-048",
    nodeType: "header4",
    content: AttributedText("Performance Requirements"),
  );

  final seedAnalyticsPerfBlock1 = BlockEntity(
    uuid: "launch-analytics-perf-049",
    nodeType: "list_item",
    content: AttributedText("<2s initial load time"),
  );

  final seedAnalyticsPerfBlock2 = BlockEntity(
    uuid: "launch-analytics-perf-050",
    nodeType: "list_item",
    content: AttributedText("<500ms for widget updates"),
  );

  final seedAnalyticsPerfBlock3 = BlockEntity(
    uuid: "launch-analytics-perf-051",
    nodeType: "list_item",
    content: AttributedText("Support 1000+ concurrent users"),
  );

  final seedExportTask = DevtoolsTaskDefaults.withDefaults(
    "01e75f3b-c679-4718-974b-46d3a561a63f",
    AttributedText("Add export functionality"),
    status: TaskStatus.started,
  );

  // Export task description blocks
  final seedExportHeaderBlock = BlockEntity(
    uuid: "launch-export-header-052",
    nodeType: "header3",
    content: AttributedText("Multi-format Export Engine 🛠️"),
  );

  final seedExportFormatsHeaderBlock = BlockEntity(
    uuid: "launch-export-formats-053",
    nodeType: "header4",
    content: AttributedText("Export Formats"),
  );

  final seedExportFormatsBlock1 = BlockEntity(
    uuid: "launch-export-formats-054",
    nodeType: "list_item",
    content: AttributedText("PDF reports with custom branding"),
  );

  final seedExportFormatsBlock2 = BlockEntity(
    uuid: "launch-export-formats-055",
    nodeType: "list_item",
    content: AttributedText("Excel spreadsheets with formulas intact"),
  );

  final seedExportFormatsBlock3 = BlockEntity(
    uuid: "launch-export-formats-056",
    nodeType: "list_item",
    content: AttributedText("CSV for data analysis"),
  );

  final seedExportFormatsBlock4 = BlockEntity(
    uuid: "launch-export-formats-057",
    nodeType: "list_item",
    content: AttributedText("PowerPoint slides for presentations"),
  );

  final seedExportFormatsBlock5 = BlockEntity(
    uuid: "launch-export-formats-058",
    nodeType: "list_item",
    content: AttributedText("RESTful API for programmatic access"),
  );

  final seedExportProgressHeaderBlock = BlockEntity(
    uuid: "launch-export-progress-059",
    nodeType: "header4",
    content: AttributedText("Current Progress"),
  );

  final seedExportProgressBlock1 = BlockEntity(
    uuid: "launch-export-progress-060",
    nodeType: "list_item",
    content: AttributedText("PDF generation: Complete"),
  );

  final seedExportProgressBlock2 = BlockEntity(
    uuid: "launch-export-progress-061",
    nodeType: "list_item",
    content: AttributedText(
      "Excel export: 60% done (working on chart embedding)",
    ),
  );

  final seedExportProgressBlock3 = BlockEntity(
    uuid: "launch-export-progress-062",
    nodeType: "list_item",
    content: AttributedText("API endpoints: In review"),
  );

  final seedExportNextHeaderBlock = BlockEntity(
    uuid: "launch-export-next-063",
    nodeType: "header4",
    content: AttributedText("Next Steps"),
  );

  final seedExportNextBlock1 = BlockEntity(
    uuid: "launch-export-next-064",
    nodeType: "list_item",
    content: AttributedText("Implement scheduled export delivery via email"),
  );

  final seedExportNextBlock2 = BlockEntity(
    uuid: "launch-export-next-065",
    nodeType: "list_item",
    content: AttributedText("Add webhook notifications for large exports"),
  );

  final seedExportNextBlock3 = BlockEntity(
    uuid: "launch-export-next-066",
    nodeType: "list_item",
    content: AttributedText("Performance optimization for datasets >100k rows"),
  );

  final seedPaymentTask = DevtoolsTaskDefaults.withDefaults(
    "10189f69-cc34-4ca1-a516-0a3684985ad7",
    AttributedText("Integrate payment processing"),
  );

  // Payment task description blocks
  final seedPaymentHeaderBlock = BlockEntity(
    uuid: "launch-payment-header-067",
    nodeType: "header3",
    content: AttributedText("Stripe Integration for Subscriptions"),
  );

  final seedPaymentFeaturesHeaderBlock = BlockEntity(
    uuid: "launch-payment-features-068",
    nodeType: "header4",
    content: AttributedText("Payment Features"),
  );

  final seedPaymentFeaturesBlock1 = BlockEntity(
    uuid: "launch-payment-features-069",
    nodeType: "list_item",
    content: AttributedText(
      "Tiered subscription plans (Starter, Pro, Enterprise)",
    ),
  );

  final seedPaymentFeaturesBlock2 = BlockEntity(
    uuid: "launch-payment-features-070",
    nodeType: "list_item",
    content: AttributedText("Annual/monthly billing with automatic discounts"),
  );

  final seedPaymentFeaturesBlock3 = BlockEntity(
    uuid: "launch-payment-features-071",
    nodeType: "list_item",
    content: AttributedText("International payment support (35+ currencies)"),
  );

  final seedPaymentFeaturesBlock4 = BlockEntity(
    uuid: "launch-payment-features-072",
    nodeType: "list_item",
    content: AttributedText("Enterprise invoicing with NET-30 terms"),
  );

  final seedPaymentFeaturesBlock5 = BlockEntity(
    uuid: "launch-payment-features-073",
    nodeType: "list_item",
    content: AttributedText("PCI DSS compliant payment flows"),
  );

  final seedPaymentImplHeaderBlock = BlockEntity(
    uuid: "launch-payment-impl-074",
    nodeType: "header4",
    content: AttributedText("Implementation"),
  );

  final seedPaymentImplBlock1 = BlockEntity(
    uuid: "launch-payment-impl-075",
    nodeType: "list_item",
    content: AttributedText("Stripe Connect for marketplace features"),
  );

  final seedPaymentImplBlock2 = BlockEntity(
    uuid: "launch-payment-impl-076",
    nodeType: "list_item",
    content: AttributedText("Webhook handling for subscription events"),
  );

  final seedPaymentImplBlock3 = BlockEntity(
    uuid: "launch-payment-impl-077",
    nodeType: "list_item",
    content: AttributedText("Tax calculation with TaxJar integration"),
  );

  final seedPaymentImplBlock4 = BlockEntity(
    uuid: "launch-payment-impl-078",
    nodeType: "list_item",
    content: AttributedText("Dunning management for failed payments"),
  );

  final seedPaymentTestingHeaderBlock = BlockEntity(
    uuid: "launch-payment-testing-079",
    nodeType: "header4",
    content: AttributedText("Testing"),
  );

  final seedPaymentTestingBlock1 = BlockEntity(
    uuid: "launch-payment-testing-080",
    nodeType: "list_item",
    content: AttributedText(
      "Sandbox environment with realistic test scenarios",
    ),
  );

  final seedPaymentTestingBlock2 = BlockEntity(
    uuid: "launch-payment-testing-081",
    nodeType: "list_item",
    content: AttributedText("Load testing for Black Friday traffic levels"),
  );

  final seedMarketingTask = DevtoolsTaskDefaults.withDefaults(
    "1cfa4479-77e1-485e-8a2c-1e358868df8a",
    AttributedText("Prepare marketing materials"),
    status: TaskStatus.started,
    dateDelta: const Duration(days: 7),
    priority: TaskPriority.H,
  );

  // Marketing task description blocks
  final seedMarketingHeaderBlock = BlockEntity(
    uuid: "launch-marketing-header-082",
    nodeType: "header3",
    content: AttributedText("Launch Marketing Campaign 🚀"),
  );

  final seedMarketingStrategyHeaderBlock = BlockEntity(
    uuid: "launch-marketing-strategy-083",
    nodeType: "header4",
    content: AttributedText("Campaign Strategy"),
  );

  final seedMarketingStrategyBlock1 = BlockEntity(
    uuid: "launch-marketing-strategy-084",
    nodeType: "list_item",
    content: AttributedText(
      "Multi-channel approach: Email, LinkedIn, Twitter, Product Hunt",
    ),
  );

  final seedMarketingStrategyBlock2 = BlockEntity(
    uuid: "launch-marketing-strategy-085",
    nodeType: "list_item",
    content: AttributedText(
      "Influencer partnerships with data science community",
    ),
  );

  final seedMarketingStrategyBlock3 = BlockEntity(
    uuid: "launch-marketing-strategy-086",
    nodeType: "list_item",
    content: AttributedText("Webinar series: \"Analytics That Actually Work\""),
  );

  final seedMarketingStrategyBlock4 = BlockEntity(
    uuid: "launch-marketing-strategy-087",
    nodeType: "list_item",
    content: AttributedText("Free trial with onboarding sequence"),
  );

  final seedMarketingContentHeaderBlock = BlockEntity(
    uuid: "launch-marketing-content-088",
    nodeType: "header4",
    content: AttributedText("Content Pipeline"),
  );

  final seedMarketingContentBlock1 = BlockEntity(
    uuid: "launch-marketing-content-089",
    nodeType: "list_item",
    content: AttributedText(
      "Product announcement btempoDBSeedLog. post (in draft)",
    ),
  );

  final seedMarketingContentBlock2 = BlockEntity(
    uuid: "launch-marketing-content-090",
    nodeType: "list_item",
    content: AttributedText("Demo video with real customer scenarios"),
  );

  final seedMarketingContentBlock3 = BlockEntity(
    uuid: "launch-marketing-content-091",
    nodeType: "list_item",
    content: AttributedText("Email sequence (7 messages over 14 days)"),
  );

  final seedMarketingContentBlock4 = BlockEntity(
    uuid: "launch-marketing-content-092",
    nodeType: "list_item",
    content: AttributedText("Social media templates and scheduling"),
  );

  final seedMarketingContentBlock5 = BlockEntity(
    uuid: "launch-marketing-content-093",
    nodeType: "list_item",
    content: AttributedText("Press kit for tech journalists"),
  );

  final seedMarketingTimelineHeaderBlock = BlockEntity(
    uuid: "launch-marketing-timeline-094",
    nodeType: "header4",
    content: AttributedText("Launch Timeline"),
  );

  final seedMarketingTimelineBlock1 = BlockEntity(
    uuid: "launch-marketing-timeline-095",
    nodeType: "list_item",
    content: AttributedText("Soft launch: March 10 (beta users)"),
  );

  final seedMarketingTimelineBlock2 = BlockEntity(
    uuid: "launch-marketing-timeline-096",
    nodeType: "list_item",
    content: AttributedText("Public launch: March 15 (general availability)"),
  );

  final seedMarketingTimelineBlock3 = BlockEntity(
    uuid: "launch-marketing-timeline-097",
    nodeType: "list_item",
    content: AttributedText("Product Hunt launch: March 16"),
  );

  final seedProductTask = DevtoolsTaskDefaults.withDefaults(
    "e2f6b210-00f3-4fee-8a01-5aaf553bb633",
    AttributedText("Write product announcement btempoDBSeedLog. post"),
    status: TaskStatus.started,
  );

  // Product task description blocks
  final seedProductHeaderBlock = BlockEntity(
    uuid: "launch-product-header-098",
    nodeType: "header3",
    content: AttributedText("Launch Announcement BtempoDBSeedLog. Post ✍️"),
  );

  final seedProductTargetBlock = BlockEntity(
    uuid: "launch-product-target-099",
    nodeType: "paragraph",
    content: AttributedText(
      "Target: 2,000 words, SEO-optimized for \"analytics dashboard\" keywords",
    ),
  );

  final seedProductOutlineHeaderBlock = BlockEntity(
    uuid: "launch-product-outline-100",
    nodeType: "header4",
    content: AttributedText("Outline"),
  );

  final seedProductOutlineBlock1 = BlockEntity(
    uuid: "launch-product-outline-101",
    nodeType: "list_item",
    content: AttributedText("Hook: The analytics problem every company faces"),
  );

  final seedProductOutlineBlock2 = BlockEntity(
    uuid: "launch-product-outline-102",
    nodeType: "list_item",
    content: AttributedText(
      "Solution: How Tempo Analytics Pro solves it differently",
    ),
  );

  final seedProductOutlineBlock3 = BlockEntity(
    uuid: "launch-product-outline-103",
    nodeType: "list_item",
    content: AttributedText(
      "Features Deep Dive: AI insights, real-time dashboards, exports",
    ),
  );

  final seedProductOutlineBlock4 = BlockEntity(
    uuid: "launch-product-outline-104",
    nodeType: "list_item",
    content: AttributedText("Customer Stories: 3 beta customer case studies"),
  );

  final seedProductOutlineBlock5 = BlockEntity(
    uuid: "launch-product-outline-105",
    nodeType: "list_item",
    content: AttributedText(
      "Technical Innovation: What makes our approach unique",
    ),
  );

  final seedProductOutlineBlock6 = BlockEntity(
    uuid: "launch-product-outline-106",
    nodeType: "list_item",
    content: AttributedText(
      "Call to Action: Free trial with onboarding support",
    ),
  );

  final seedProductStatusHeaderBlock = BlockEntity(
    uuid: "launch-product-status-107",
    nodeType: "header4",
    content: AttributedText("Current Status"),
  );

  final seedProductStatusBlock1 = BlockEntity(
    uuid: "launch-product-status-108",
    nodeType: "list_item",
    content: AttributedText("First draft: 75% complete"),
  );

  final seedProductStatusBlock2 = BlockEntity(
    uuid: "launch-product-status-109",
    nodeType: "list_item",
    content: AttributedText("Customer quotes: Collected from 2/3 case studies"),
  );

  final seedProductStatusBlock3 = BlockEntity(
    uuid: "launch-product-status-110",
    nodeType: "list_item",
    content: AttributedText("Screenshots: Waiting for final UI polish"),
  );

  final seedProductStatusBlock4 = BlockEntity(
    uuid: "launch-product-status-111",
    nodeType: "list_item",
    content: AttributedText("SEO review: Scheduled for Monday"),
  );

  final seedProductDistributionBlock = BlockEntity(
    uuid: "launch-product-distribution-112",
    nodeType: "paragraph",
    content: AttributedText(
      "Distribution: Company btempoDBSeedLog., Medium, LinkedIn, dev.to",
    ),
  );

  final seedDemoTask = DevtoolsTaskDefaults.withDefaults(
    "e8e62978-a036-4901-ad40-b305a60e1110",
    AttributedText("Create demo video script"),
  );

  // Demo task description blocks
  final seedDemoHeaderBlock = BlockEntity(
    uuid: "launch-demo-header-113",
    nodeType: "header3",
    content: AttributedText("Product Demo Video (3-5 minutes)"),
  );

  final seedDemoConceptBlock = BlockEntity(
    uuid: "launch-demo-concept-114",
    nodeType: "paragraph",
    content: AttributedText(
      "Video Concept: \"From Data Chaos to Insights in 3 Minutes\"",
    ),
  );

  final seedDemoScriptHeaderBlock = BlockEntity(
    uuid: "launch-demo-script-115",
    nodeType: "header4",
    content: AttributedText("Script Structure"),
  );

  final seedDemoScriptBlock1 = BlockEntity(
    uuid: "launch-demo-script-116",
    nodeType: "list_item",
    content: AttributedText(
      "Problem Setup (30s): Scattered dashboards, manual reporting",
    ),
  );

  final seedDemoScriptBlock2 = BlockEntity(
    uuid: "launch-demo-script-117",
    nodeType: "list_item",
    content: AttributedText(
      "Solution Intro (45s): Tempo Analytics Pro overview",
    ),
  );

  final seedDemoScriptBlock3 = BlockEntity(
    uuid: "launch-demo-script-118",
    nodeType: "list_item",
    content: AttributedText(
      "Feature Walkthrough (2m): Live demo of key features",
    ),
  );

  final seedDemoScriptSubBlock1 = BlockEntity(
    uuid: "launch-demo-script-119",
    nodeType: "paragraph",
    content: AttributedText(
      "  - Dashboard customization\n  - AI insights in action\n  - One-click export to stakeholders",
    ),
  );

  final seedDemoScriptBlock4 = BlockEntity(
    uuid: "launch-demo-script-120",
    nodeType: "list_item",
    content: AttributedText("Customer Success (30s): Real ROI numbers"),
  );

  final seedDemoScriptBlock5 = BlockEntity(
    uuid: "launch-demo-script-121",
    nodeType: "list_item",
    content: AttributedText("Call to Action (15s): Start free trial"),
  );

  final seedDemoProductionHeaderBlock = BlockEntity(
    uuid: "launch-demo-production-122",
    nodeType: "header4",
    content: AttributedText("Production Notes"),
  );

  final seedDemoProductionBlock1 = BlockEntity(
    uuid: "launch-demo-production-123",
    nodeType: "list_item",
    content: AttributedText(
      "Screen recording with realistic (but anonymized) data",
    ),
  );

  final seedDemoProductionBlock2 = BlockEntity(
    uuid: "launch-demo-production-124",
    nodeType: "list_item",
    content: AttributedText("Professional voiceover + background music"),
  );

  final seedDemoProductionBlock3 = BlockEntity(
    uuid: "launch-demo-production-125",
    nodeType: "list_item",
    content: AttributedText("Captions for accessibility"),
  );

  final seedDemoProductionBlock4 = BlockEntity(
    uuid: "launch-demo-production-126",
    nodeType: "list_item",
    content: AttributedText(
      "Multiple versions: 30s (social), 60s (ads), 3m (product page)",
    ),
  );

  final seedEmailTask = DevtoolsTaskDefaults.withDefaults(
    "ff4d1126-7ba5-4cb3-8f4c-9b718335ada1",
    AttributedText("Design email campaign templates"),
  );

  // Email task description blocks
  final seedEmailHeaderBlock = BlockEntity(
    uuid: "launch-email-header-127",
    nodeType: "header3",
    content: AttributedText("Email Campaign Template Suite"),
  );

  final seedEmailSequenceHeaderBlock = BlockEntity(
    uuid: "launch-email-sequence-128",
    nodeType: "header4",
    content: AttributedText("7-Email Launch Sequence"),
  );

  final seedEmailSequenceBlock1 = BlockEntity(
    uuid: "launch-email-sequence-129",
    nodeType: "list_item",
    content: AttributedText(
      "Welcome & Setup - Get started guide with quick wins",
    ),
  );

  final seedEmailSequenceBlock2 = BlockEntity(
    uuid: "launch-email-sequence-130",
    nodeType: "list_item",
    content: AttributedText("Feature Spotlight - Dashboard customization tips"),
  );

  final seedEmailSequenceBlock3 = BlockEntity(
    uuid: "launch-email-sequence-131",
    nodeType: "list_item",
    content: AttributedText(
      "AI Insights Explained - How machine learning finds patterns",
    ),
  );

  final seedEmailSequenceBlock4 = BlockEntity(
    uuid: "launch-email-sequence-132",
    nodeType: "list_item",
    content: AttributedText(
      "Export Mastery - Professional reporting workflows",
    ),
  );

  final seedEmailSequenceBlock5 = BlockEntity(
    uuid: "launch-email-sequence-133",
    nodeType: "list_item",
    content: AttributedText(
      "Customer Success Story - Case study with concrete ROI",
    ),
  );

  final seedEmailSequenceBlock6 = BlockEntity(
    uuid: "launch-email-sequence-134",
    nodeType: "list_item",
    content: AttributedText("Advanced Features - Power user tips and tricks"),
  );

  final seedEmailSequenceBlock7 = BlockEntity(
    uuid: "launch-email-sequence-135",
    nodeType: "list_item",
    content: AttributedText(
      "Upgrade Prompt - Trial expiring, upgrade benefits",
    ),
  );

  final seedEmailDesignHeaderBlock = BlockEntity(
    uuid: "launch-email-design-136",
    nodeType: "header4",
    content: AttributedText("Design Requirements"),
  );

  final seedEmailDesignBlock1 = BlockEntity(
    uuid: "launch-email-design-137",
    nodeType: "list_item",
    content: AttributedText("Mobile-responsive templates"),
  );

  final seedEmailDesignBlock2 = BlockEntity(
    uuid: "launch-email-design-138",
    nodeType: "list_item",
    content: AttributedText("Consistent brand colors and typography"),
  );

  final seedEmailDesignBlock3 = BlockEntity(
    uuid: "launch-email-design-139",
    nodeType: "list_item",
    content: AttributedText("Clear CTAs with A/B testing variants"),
  );

  final seedEmailDesignBlock4 = BlockEntity(
    uuid: "launch-email-design-140",
    nodeType: "list_item",
    content: AttributedText(
      "Personalization tokens (name, company, trial status)",
    ),
  );

  final seedEmailTechHeaderBlock = BlockEntity(
    uuid: "launch-email-tech-141",
    nodeType: "header4",
    content: AttributedText("Technical Setup"),
  );

  final seedEmailTechBlock1 = BlockEntity(
    uuid: "launch-email-tech-142",
    nodeType: "list_item",
    content: AttributedText("Mailchimp automation flows"),
  );

  final seedEmailTechBlock2 = BlockEntity(
    uuid: "launch-email-tech-143",
    nodeType: "list_item",
    content: AttributedText("UTM tracking for attribution"),
  );

  final seedEmailTechBlock3 = BlockEntity(
    uuid: "launch-email-tech-144",
    nodeType: "list_item",
    content: AttributedText("Deliverability optimization"),
  );

  @override
  Future<void> seed(SeedDataService s) async {
    tempoDBSeedLog.finer(
      '  <seeding project="${seedProject.title.toPlainText()}">',
    );
    final project = await s.projectRepo.putAsync(seedProject);

    // For this complex file, keeping simplified approach with putAsync for main tasks
    final featureDevTask = await s.taskRepo.putAsync(seedFeatureDevTask);
    final marketingTask = await s.taskRepo.putAsync(seedMarketingTask);

    project.thing.childrenRef.addAll([featureDevTask, marketingTask].things);
    await s.projectRepo.putAsync(project);
  }
}
