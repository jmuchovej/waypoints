import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project_entity.dart';
import '../../../things/task/task_entity.dart';
import '../../../things/block/block_entity.dart';
import '../seed.dart';
import '_seed.dart';

class Q1LaunchProject implements DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("Q1 Product Launch"),
    uuid: "ad8fd8d8-974c-49ff-bbfc-b474ccca3467",
  );

  // Project description blocks
  final projectHeaderBlock = BlockEntity(
    uuid: "launch-proj-header-001",
    nodeType: "header1",
    content: AttributedText("Q1 2025 Product Launch"),
  );

  final projectDetailsBlock = BlockEntity(
    uuid: "launch-proj-details-002",
    nodeType: "paragraph",
    content: AttributedText("Launch Date: March 15, 2025 | Product: Tempo Analytics Pro - Advanced analytics dashboard with AI insights"),
  );

  final strategyHeaderBlock = BlockEntity(
    uuid: "launch-proj-strategy-003",
    nodeType: "header2",
    content: AttributedText("Launch Strategy"),
  );

  final strategyBlock = BlockEntity(
    uuid: "launch-proj-strategy-004",
    nodeType: "paragraph",
    content: AttributedText("Coordinated product and marketing launch targeting enterprise customers. Focus on demonstrating ROI through automated insights and custom reporting capabilities."),
  );

  final featuresHeaderBlock = BlockEntity(
    uuid: "launch-proj-features-005",
    nodeType: "header2", 
    content: AttributedText("Key Features"),
  );

  final featuresListBlock1 = BlockEntity(
    uuid: "launch-proj-features-006",
    nodeType: "list_item",
    content: AttributedText("Real-time Dashboard: Live metrics with customizable widgets"),
  );

  final featuresListBlock2 = BlockEntity(
    uuid: "launch-proj-features-007",
    nodeType: "list_item",
    content: AttributedText("AI Insights: Automated anomaly detection and recommendations"),
  );

  final featuresListBlock3 = BlockEntity(
    uuid: "launch-proj-features-008",
    nodeType: "list_item",
    content: AttributedText("Export Engine: Support for PDF, Excel, and API integrations"),
  );

  final featuresListBlock4 = BlockEntity(
    uuid: "launch-proj-features-009",
    nodeType: "list_item",
    content: AttributedText("Enterprise Auth: SSO, RBAC, and audit logging"),
  );

  final metricsHeaderBlock = BlockEntity(
    uuid: "launch-proj-metrics-010",
    nodeType: "header2",
    content: AttributedText("Success Metrics"),
  );

  final metricsListBlock1 = BlockEntity(
    uuid: "launch-proj-metrics-011",
    nodeType: "list_item",
    content: AttributedText("500 beta signups in first week"),
  );

  final metricsListBlock2 = BlockEntity(
    uuid: "launch-proj-metrics-012",
    nodeType: "list_item",
    content: AttributedText("50 enterprise demos booked"),
  );

  final metricsListBlock3 = BlockEntity(
    uuid: "launch-proj-metrics-013",
    nodeType: "list_item",
    content: AttributedText("15% conversion rate from trial to paid"),
  );

  final metricsListBlock4 = BlockEntity(
    uuid: "launch-proj-metrics-014",
    nodeType: "list_item",
    content: AttributedText("Featured in 3 major industry publications"),
  );

  final featureDevTask = DevtoolsTaskDefaults.withDefaults(
    "906ed45c-c56d-45a6-b24c-f109f90155d9",
    AttributedText("Complete feature development"),
    status: TaskStatus.workInProgress,
    dateDelta: const Duration(days: -2),
    priority: TaskPriority.H,
  );

  // Feature development task description blocks
  final featureDevHeaderBlock = BlockEntity(
    uuid: "launch-featuredev-header-015",
    nodeType: "header3",
    content: AttributedText("Final development sprint for Tempo Analytics Pro"),
  );

  final featureDevStatusBlock = BlockEntity(
    uuid: "launch-featuredev-status-016",
    nodeType: "paragraph",
    content: AttributedText("Current Status: 75% complete, 2 days overdue"),
  );

  final featureDevWorkHeaderBlock = BlockEntity(
    uuid: "launch-featuredev-work-017",
    nodeType: "header4",
    content: AttributedText("Remaining Work"),
  );

  final featureDevWorkBlock1 = BlockEntity(
    uuid: "launch-featuredev-work-018",
    nodeType: "list_item",
    content: AttributedText("Finalize AI insights algorithm (ML team)"),
  );

  final featureDevWorkBlock2 = BlockEntity(
    uuid: "launch-featuredev-work-019",
    nodeType: "list_item",
    content: AttributedText("Complete enterprise authentication integration"),
  );

  final featureDevWorkBlock3 = BlockEntity(
    uuid: "launch-featuredev-work-020",
    nodeType: "list_item",
    content: AttributedText("Polish dashboard performance for 1000+ concurrent users"),
  );

  final featureDevBlockersBlock = BlockEntity(
    uuid: "launch-featuredev-blockers-021",
    nodeType: "paragraph",
    content: AttributedText("Blockers: AWS infrastructure scaling approval pending, security audit findings need resolution."),
  );

  final featureDevMeetingsBlock = BlockEntity(
    uuid: "launch-featuredev-meetings-022",
    nodeType: "paragraph",
    content: AttributedText("Daily standups at 9:00 AM, all-hands demo Friday."),
  );

  final userAuthTask = DevtoolsTaskDefaults.withDefaults(
    "265a8582-aba3-48a5-97c1-964a8d749db7",
    AttributedText("Implement user authentication flow"),
    status: TaskStatus.complete,
  );

  // User auth task description blocks
  final userAuthHeaderBlock = BlockEntity(
    uuid: "launch-userauth-header-023",
    nodeType: "header3",
    content: AttributedText("Enterprise-grade authentication system ✅"),
  );

  final userAuthFeaturesHeaderBlock = BlockEntity(
    uuid: "launch-userauth-features-024",
    nodeType: "header4",
    content: AttributedText("Implemented Features"),
  );

  final userAuthFeaturesBlock1 = BlockEntity(
    uuid: "launch-userauth-features-025",
    nodeType: "list_item",
    content: AttributedText("SAML 2.0 and OIDC support"),
  );

  final userAuthFeaturesBlock2 = BlockEntity(
    uuid: "launch-userauth-features-026",
    nodeType: "list_item",
    content: AttributedText("Role-based access control (RBAC)"),
  );

  final userAuthFeaturesBlock3 = BlockEntity(
    uuid: "launch-userauth-features-027",
    nodeType: "list_item",
    content: AttributedText("Multi-factor authentication"),
  );

  final userAuthFeaturesBlock4 = BlockEntity(
    uuid: "launch-userauth-features-028",
    nodeType: "list_item",
    content: AttributedText("Session management with Redis"),
  );

  final userAuthFeaturesBlock5 = BlockEntity(
    uuid: "launch-userauth-features-029",
    nodeType: "list_item",
    content: AttributedText("Audit logging for compliance"),
  );

  final userAuthSecurityHeaderBlock = BlockEntity(
    uuid: "launch-userauth-security-030",
    nodeType: "header4",
    content: AttributedText("Security Features"),
  );

  final userAuthSecurityBlock1 = BlockEntity(
    uuid: "launch-userauth-security-031",
    nodeType: "list_item",
    content: AttributedText("JWT tokens with 15-minute expiry"),
  );

  final userAuthSecurityBlock2 = BlockEntity(
    uuid: "launch-userauth-security-032",
    nodeType: "list_item",
    content: AttributedText("Refresh token rotation"),
  );

  final userAuthSecurityBlock3 = BlockEntity(
    uuid: "launch-userauth-security-033",
    nodeType: "list_item",
    content: AttributedText("Rate limiting on auth endpoints"),
  );

  final userAuthSecurityBlock4 = BlockEntity(
    uuid: "launch-userauth-security-034",
    nodeType: "list_item",
    content: AttributedText("Password policy enforcement"),
  );

  final userAuthTestingBlock = BlockEntity(
    uuid: "launch-userauth-testing-035",
    nodeType: "paragraph",
    content: AttributedText("Testing: Passed penetration testing and SOC 2 Type II requirements."),
  );

  final analyticsDashboardTask = DevtoolsTaskDefaults.withDefaults(
    "4110eaee-405d-452f-b5bd-9ceac2af094f",
    AttributedText("Build dashboard analytics view"),
  );

  // Analytics dashboard task description blocks
  final analyticsHeaderBlock = BlockEntity(
    uuid: "launch-analytics-header-036",
    nodeType: "header3",
    content: AttributedText("Interactive Analytics Dashboard"),
  );

  final analyticsFeaturesHeaderBlock = BlockEntity(
    uuid: "launch-analytics-features-037",
    nodeType: "header4",
    content: AttributedText("Core Features"),
  );

  final analyticsFeaturesBlock1 = BlockEntity(
    uuid: "launch-analytics-features-038",
    nodeType: "list_item",
    content: AttributedText("Drag-and-drop widget customization"),
  );

  final analyticsFeaturesBlock2 = BlockEntity(
    uuid: "launch-analytics-features-039",
    nodeType: "list_item",
    content: AttributedText("Real-time data streaming with WebSockets"),
  );

  final analyticsFeaturesBlock3 = BlockEntity(
    uuid: "launch-analytics-features-040",
    nodeType: "list_item",
    content: AttributedText("15+ chart types (line, bar, heatmap, funnel)"),
  );

  final analyticsFeaturesBlock4 = BlockEntity(
    uuid: "launch-analytics-features-041",
    nodeType: "list_item",
    content: AttributedText("Custom date range filtering"),
  );

  final analyticsFeaturesBlock5 = BlockEntity(
    uuid: "launch-analytics-features-042",
    nodeType: "list_item",
    content: AttributedText("Export to PNG/PDF with branded templates"),
  );

  final analyticsTechHeaderBlock = BlockEntity(
    uuid: "launch-analytics-tech-043",
    nodeType: "header4",
    content: AttributedText("Technical Stack"),
  );

  final analyticsTechBlock1 = BlockEntity(
    uuid: "launch-analytics-tech-044",
    nodeType: "list_item",
    content: AttributedText("React + TypeScript frontend"),
  );

  final analyticsTechBlock2 = BlockEntity(
    uuid: "launch-analytics-tech-045",
    nodeType: "list_item",
    content: AttributedText("D3.js for custom visualizations"),
  );

  final analyticsTechBlock3 = BlockEntity(
    uuid: "launch-analytics-tech-046",
    nodeType: "list_item",
    content: AttributedText("Apache Kafka for real-time streams"),
  );

  final analyticsTechBlock4 = BlockEntity(
    uuid: "launch-analytics-tech-047",
    nodeType: "list_item",
    content: AttributedText("Redis for caching frequently accessed data"),
  );

  final analyticsPerfHeaderBlock = BlockEntity(
    uuid: "launch-analytics-perf-048",
    nodeType: "header4",
    content: AttributedText("Performance Requirements"),
  );

  final analyticsPerfBlock1 = BlockEntity(
    uuid: "launch-analytics-perf-049",
    nodeType: "list_item",
    content: AttributedText("<2s initial load time"),
  );

  final analyticsPerfBlock2 = BlockEntity(
    uuid: "launch-analytics-perf-050",
    nodeType: "list_item",
    content: AttributedText("<500ms for widget updates"),
  );

  final analyticsPerfBlock3 = BlockEntity(
    uuid: "launch-analytics-perf-051",
    nodeType: "list_item",
    content: AttributedText("Support 1000+ concurrent users"),
  );

  final exportTask = DevtoolsTaskDefaults.withDefaults(
    "01e75f3b-c679-4718-974b-46d3a561a63f",
    AttributedText("Add export functionality"),
    status: TaskStatus.started,
  );

  // Export task description blocks
  final exportHeaderBlock = BlockEntity(
    uuid: "launch-export-header-052",
    nodeType: "header3",
    content: AttributedText("Multi-format Export Engine 🛠️"),
  );

  final exportFormatsHeaderBlock = BlockEntity(
    uuid: "launch-export-formats-053",
    nodeType: "header4",
    content: AttributedText("Export Formats"),
  );

  final exportFormatsBlock1 = BlockEntity(
    uuid: "launch-export-formats-054",
    nodeType: "list_item",
    content: AttributedText("PDF reports with custom branding"),
  );

  final exportFormatsBlock2 = BlockEntity(
    uuid: "launch-export-formats-055",
    nodeType: "list_item",
    content: AttributedText("Excel spreadsheets with formulas intact"),
  );

  final exportFormatsBlock3 = BlockEntity(
    uuid: "launch-export-formats-056",
    nodeType: "list_item",
    content: AttributedText("CSV for data analysis"),
  );

  final exportFormatsBlock4 = BlockEntity(
    uuid: "launch-export-formats-057",
    nodeType: "list_item",
    content: AttributedText("PowerPoint slides for presentations"),
  );

  final exportFormatsBlock5 = BlockEntity(
    uuid: "launch-export-formats-058",
    nodeType: "list_item",
    content: AttributedText("RESTful API for programmatic access"),
  );

  final exportProgressHeaderBlock = BlockEntity(
    uuid: "launch-export-progress-059",
    nodeType: "header4",
    content: AttributedText("Current Progress"),
  );

  final exportProgressBlock1 = BlockEntity(
    uuid: "launch-export-progress-060",
    nodeType: "list_item",
    content: AttributedText("PDF generation: Complete"),
  );

  final exportProgressBlock2 = BlockEntity(
    uuid: "launch-export-progress-061",
    nodeType: "list_item",
    content: AttributedText("Excel export: 60% done (working on chart embedding)"),
  );

  final exportProgressBlock3 = BlockEntity(
    uuid: "launch-export-progress-062",
    nodeType: "list_item",
    content: AttributedText("API endpoints: In review"),
  );

  final exportNextHeaderBlock = BlockEntity(
    uuid: "launch-export-next-063",
    nodeType: "header4",
    content: AttributedText("Next Steps"),
  );

  final exportNextBlock1 = BlockEntity(
    uuid: "launch-export-next-064",
    nodeType: "list_item",
    content: AttributedText("Implement scheduled export delivery via email"),
  );

  final exportNextBlock2 = BlockEntity(
    uuid: "launch-export-next-065",
    nodeType: "list_item",
    content: AttributedText("Add webhook notifications for large exports"),
  );

  final exportNextBlock3 = BlockEntity(
    uuid: "launch-export-next-066",
    nodeType: "list_item",
    content: AttributedText("Performance optimization for datasets >100k rows"),
  );

  final paymentTask = DevtoolsTaskDefaults.withDefaults(
    "10189f69-cc34-4ca1-a516-0a3684985ad7",
    AttributedText("Integrate payment processing"),
  );

  // Payment task description blocks
  final paymentHeaderBlock = BlockEntity(
    uuid: "launch-payment-header-067",
    nodeType: "header3",
    content: AttributedText("Stripe Integration for Subscriptions"),
  );

  final paymentFeaturesHeaderBlock = BlockEntity(
    uuid: "launch-payment-features-068",
    nodeType: "header4",
    content: AttributedText("Payment Features"),
  );

  final paymentFeaturesBlock1 = BlockEntity(
    uuid: "launch-payment-features-069",
    nodeType: "list_item",
    content: AttributedText("Tiered subscription plans (Starter, Pro, Enterprise)"),
  );

  final paymentFeaturesBlock2 = BlockEntity(
    uuid: "launch-payment-features-070",
    nodeType: "list_item",
    content: AttributedText("Annual/monthly billing with automatic discounts"),
  );

  final paymentFeaturesBlock3 = BlockEntity(
    uuid: "launch-payment-features-071",
    nodeType: "list_item",
    content: AttributedText("International payment support (35+ currencies)"),
  );

  final paymentFeaturesBlock4 = BlockEntity(
    uuid: "launch-payment-features-072",
    nodeType: "list_item",
    content: AttributedText("Enterprise invoicing with NET-30 terms"),
  );

  final paymentFeaturesBlock5 = BlockEntity(
    uuid: "launch-payment-features-073",
    nodeType: "list_item",
    content: AttributedText("PCI DSS compliant payment flows"),
  );

  final paymentImplHeaderBlock = BlockEntity(
    uuid: "launch-payment-impl-074",
    nodeType: "header4",
    content: AttributedText("Implementation"),
  );

  final paymentImplBlock1 = BlockEntity(
    uuid: "launch-payment-impl-075",
    nodeType: "list_item",
    content: AttributedText("Stripe Connect for marketplace features"),
  );

  final paymentImplBlock2 = BlockEntity(
    uuid: "launch-payment-impl-076",
    nodeType: "list_item",
    content: AttributedText("Webhook handling for subscription events"),
  );

  final paymentImplBlock3 = BlockEntity(
    uuid: "launch-payment-impl-077",
    nodeType: "list_item",
    content: AttributedText("Tax calculation with TaxJar integration"),
  );

  final paymentImplBlock4 = BlockEntity(
    uuid: "launch-payment-impl-078",
    nodeType: "list_item",
    content: AttributedText("Dunning management for failed payments"),
  );

  final paymentTestingHeaderBlock = BlockEntity(
    uuid: "launch-payment-testing-079",
    nodeType: "header4",
    content: AttributedText("Testing"),
  );

  final paymentTestingBlock1 = BlockEntity(
    uuid: "launch-payment-testing-080",
    nodeType: "list_item",
    content: AttributedText("Sandbox environment with realistic test scenarios"),
  );

  final paymentTestingBlock2 = BlockEntity(
    uuid: "launch-payment-testing-081",
    nodeType: "list_item",
    content: AttributedText("Load testing for Black Friday traffic levels"),
  );

  final marketingTask = DevtoolsTaskDefaults.withDefaults(
    "1cfa4479-77e1-485e-8a2c-1e358868df8a",
    AttributedText("Prepare marketing materials"),
    status: TaskStatus.started,
    dateDelta: const Duration(days: 7),
    priority: TaskPriority.H,
  );

  // Marketing task description blocks
  final marketingHeaderBlock = BlockEntity(
    uuid: "launch-marketing-header-082",
    nodeType: "header3",
    content: AttributedText("Launch Marketing Campaign 🚀"),
  );

  final marketingStrategyHeaderBlock = BlockEntity(
    uuid: "launch-marketing-strategy-083",
    nodeType: "header4",
    content: AttributedText("Campaign Strategy"),
  );

  final marketingStrategyBlock1 = BlockEntity(
    uuid: "launch-marketing-strategy-084",
    nodeType: "list_item",
    content: AttributedText("Multi-channel approach: Email, LinkedIn, Twitter, Product Hunt"),
  );

  final marketingStrategyBlock2 = BlockEntity(
    uuid: "launch-marketing-strategy-085",
    nodeType: "list_item",
    content: AttributedText("Influencer partnerships with data science community"),
  );

  final marketingStrategyBlock3 = BlockEntity(
    uuid: "launch-marketing-strategy-086",
    nodeType: "list_item",
    content: AttributedText("Webinar series: \"Analytics That Actually Work\""),
  );

  final marketingStrategyBlock4 = BlockEntity(
    uuid: "launch-marketing-strategy-087",
    nodeType: "list_item",
    content: AttributedText("Free trial with onboarding sequence"),
  );

  final marketingContentHeaderBlock = BlockEntity(
    uuid: "launch-marketing-content-088",
    nodeType: "header4",
    content: AttributedText("Content Pipeline"),
  );

  final marketingContentBlock1 = BlockEntity(
    uuid: "launch-marketing-content-089",
    nodeType: "list_item",
    content: AttributedText("Product announcement blog post (in draft)"),
  );

  final marketingContentBlock2 = BlockEntity(
    uuid: "launch-marketing-content-090",
    nodeType: "list_item",
    content: AttributedText("Demo video with real customer scenarios"),
  );

  final marketingContentBlock3 = BlockEntity(
    uuid: "launch-marketing-content-091",
    nodeType: "list_item",
    content: AttributedText("Email sequence (7 messages over 14 days)"),
  );

  final marketingContentBlock4 = BlockEntity(
    uuid: "launch-marketing-content-092",
    nodeType: "list_item",
    content: AttributedText("Social media templates and scheduling"),
  );

  final marketingContentBlock5 = BlockEntity(
    uuid: "launch-marketing-content-093",
    nodeType: "list_item",
    content: AttributedText("Press kit for tech journalists"),
  );

  final marketingTimelineHeaderBlock = BlockEntity(
    uuid: "launch-marketing-timeline-094",
    nodeType: "header4",
    content: AttributedText("Launch Timeline"),
  );

  final marketingTimelineBlock1 = BlockEntity(
    uuid: "launch-marketing-timeline-095",
    nodeType: "list_item",
    content: AttributedText("Soft launch: March 10 (beta users)"),
  );

  final marketingTimelineBlock2 = BlockEntity(
    uuid: "launch-marketing-timeline-096",
    nodeType: "list_item",
    content: AttributedText("Public launch: March 15 (general availability)"),
  );

  final marketingTimelineBlock3 = BlockEntity(
    uuid: "launch-marketing-timeline-097",
    nodeType: "list_item",
    content: AttributedText("Product Hunt launch: March 16"),
  );

  final productTask = DevtoolsTaskDefaults.withDefaults(
    "e2f6b210-00f3-4fee-8a01-5aaf553bb633",
    AttributedText("Write product announcement blog post"),
    status: TaskStatus.started,
  );

  // Product task description blocks
  final productHeaderBlock = BlockEntity(
    uuid: "launch-product-header-098",
    nodeType: "header3",
    content: AttributedText("Launch Announcement Blog Post ✍️"),
  );

  final productTargetBlock = BlockEntity(
    uuid: "launch-product-target-099",
    nodeType: "paragraph",
    content: AttributedText("Target: 2,000 words, SEO-optimized for \"analytics dashboard\" keywords"),
  );

  final productOutlineHeaderBlock = BlockEntity(
    uuid: "launch-product-outline-100",
    nodeType: "header4",
    content: AttributedText("Outline"),
  );

  final productOutlineBlock1 = BlockEntity(
    uuid: "launch-product-outline-101",
    nodeType: "list_item",
    content: AttributedText("Hook: The analytics problem every company faces"),
  );

  final productOutlineBlock2 = BlockEntity(
    uuid: "launch-product-outline-102",
    nodeType: "list_item",
    content: AttributedText("Solution: How Tempo Analytics Pro solves it differently"),
  );

  final productOutlineBlock3 = BlockEntity(
    uuid: "launch-product-outline-103",
    nodeType: "list_item",
    content: AttributedText("Features Deep Dive: AI insights, real-time dashboards, exports"),
  );

  final productOutlineBlock4 = BlockEntity(
    uuid: "launch-product-outline-104",
    nodeType: "list_item",
    content: AttributedText("Customer Stories: 3 beta customer case studies"),
  );

  final productOutlineBlock5 = BlockEntity(
    uuid: "launch-product-outline-105",
    nodeType: "list_item",
    content: AttributedText("Technical Innovation: What makes our approach unique"),
  );

  final productOutlineBlock6 = BlockEntity(
    uuid: "launch-product-outline-106",
    nodeType: "list_item",
    content: AttributedText("Call to Action: Free trial with onboarding support"),
  );

  final productStatusHeaderBlock = BlockEntity(
    uuid: "launch-product-status-107",
    nodeType: "header4",
    content: AttributedText("Current Status"),
  );

  final productStatusBlock1 = BlockEntity(
    uuid: "launch-product-status-108",
    nodeType: "list_item",
    content: AttributedText("First draft: 75% complete"),
  );

  final productStatusBlock2 = BlockEntity(
    uuid: "launch-product-status-109",
    nodeType: "list_item",
    content: AttributedText("Customer quotes: Collected from 2/3 case studies"),
  );

  final productStatusBlock3 = BlockEntity(
    uuid: "launch-product-status-110",
    nodeType: "list_item",
    content: AttributedText("Screenshots: Waiting for final UI polish"),
  );

  final productStatusBlock4 = BlockEntity(
    uuid: "launch-product-status-111",
    nodeType: "list_item",
    content: AttributedText("SEO review: Scheduled for Monday"),
  );

  final productDistributionBlock = BlockEntity(
    uuid: "launch-product-distribution-112",
    nodeType: "paragraph",
    content: AttributedText("Distribution: Company blog, Medium, LinkedIn, dev.to"),
  );

  final demoTask = DevtoolsTaskDefaults.withDefaults(
    "e8e62978-a036-4901-ad40-b305a60e1110",
    AttributedText("Create demo video script"),
  );

  // Demo task description blocks
  final demoHeaderBlock = BlockEntity(
    uuid: "launch-demo-header-113",
    nodeType: "header3",
    content: AttributedText("Product Demo Video (3-5 minutes)"),
  );

  final demoConceptBlock = BlockEntity(
    uuid: "launch-demo-concept-114",
    nodeType: "paragraph",
    content: AttributedText("Video Concept: \"From Data Chaos to Insights in 3 Minutes\""),
  );

  final demoScriptHeaderBlock = BlockEntity(
    uuid: "launch-demo-script-115",
    nodeType: "header4",
    content: AttributedText("Script Structure"),
  );

  final demoScriptBlock1 = BlockEntity(
    uuid: "launch-demo-script-116",
    nodeType: "list_item",
    content: AttributedText("Problem Setup (30s): Scattered dashboards, manual reporting"),
  );

  final demoScriptBlock2 = BlockEntity(
    uuid: "launch-demo-script-117",
    nodeType: "list_item",
    content: AttributedText("Solution Intro (45s): Tempo Analytics Pro overview"),
  );

  final demoScriptBlock3 = BlockEntity(
    uuid: "launch-demo-script-118",
    nodeType: "list_item",
    content: AttributedText("Feature Walkthrough (2m): Live demo of key features"),
  );

  final demoScriptSubBlock1 = BlockEntity(
    uuid: "launch-demo-script-119",
    nodeType: "paragraph",
    content: AttributedText("  - Dashboard customization\n  - AI insights in action\n  - One-click export to stakeholders"),
  );

  final demoScriptBlock4 = BlockEntity(
    uuid: "launch-demo-script-120",
    nodeType: "list_item",
    content: AttributedText("Customer Success (30s): Real ROI numbers"),
  );

  final demoScriptBlock5 = BlockEntity(
    uuid: "launch-demo-script-121",
    nodeType: "list_item",
    content: AttributedText("Call to Action (15s): Start free trial"),
  );

  final demoProductionHeaderBlock = BlockEntity(
    uuid: "launch-demo-production-122",
    nodeType: "header4",
    content: AttributedText("Production Notes"),
  );

  final demoProductionBlock1 = BlockEntity(
    uuid: "launch-demo-production-123",
    nodeType: "list_item",
    content: AttributedText("Screen recording with realistic (but anonymized) data"),
  );

  final demoProductionBlock2 = BlockEntity(
    uuid: "launch-demo-production-124",
    nodeType: "list_item",
    content: AttributedText("Professional voiceover + background music"),
  );

  final demoProductionBlock3 = BlockEntity(
    uuid: "launch-demo-production-125",
    nodeType: "list_item",
    content: AttributedText("Captions for accessibility"),
  );

  final demoProductionBlock4 = BlockEntity(
    uuid: "launch-demo-production-126",
    nodeType: "list_item",
    content: AttributedText("Multiple versions: 30s (social), 60s (ads), 3m (product page)"),
  );

  final emailTask = DevtoolsTaskDefaults.withDefaults(
    "ff4d1126-7ba5-4cb3-8f4c-9b718335ada1",
    AttributedText("Design email campaign templates"),
  );

  // Email task description blocks
  final emailHeaderBlock = BlockEntity(
    uuid: "launch-email-header-127",
    nodeType: "header3",
    content: AttributedText("Email Campaign Template Suite"),
  );

  final emailSequenceHeaderBlock = BlockEntity(
    uuid: "launch-email-sequence-128",
    nodeType: "header4",
    content: AttributedText("7-Email Launch Sequence"),
  );

  final emailSequenceBlock1 = BlockEntity(
    uuid: "launch-email-sequence-129",
    nodeType: "list_item",
    content: AttributedText("Welcome & Setup - Get started guide with quick wins"),
  );

  final emailSequenceBlock2 = BlockEntity(
    uuid: "launch-email-sequence-130",
    nodeType: "list_item",
    content: AttributedText("Feature Spotlight - Dashboard customization tips"),
  );

  final emailSequenceBlock3 = BlockEntity(
    uuid: "launch-email-sequence-131",
    nodeType: "list_item",
    content: AttributedText("AI Insights Explained - How machine learning finds patterns"),
  );

  final emailSequenceBlock4 = BlockEntity(
    uuid: "launch-email-sequence-132",
    nodeType: "list_item",
    content: AttributedText("Export Mastery - Professional reporting workflows"),
  );

  final emailSequenceBlock5 = BlockEntity(
    uuid: "launch-email-sequence-133",
    nodeType: "list_item",
    content: AttributedText("Customer Success Story - Case study with concrete ROI"),
  );

  final emailSequenceBlock6 = BlockEntity(
    uuid: "launch-email-sequence-134",
    nodeType: "list_item",
    content: AttributedText("Advanced Features - Power user tips and tricks"),
  );

  final emailSequenceBlock7 = BlockEntity(
    uuid: "launch-email-sequence-135",
    nodeType: "list_item",
    content: AttributedText("Upgrade Prompt - Trial expiring, upgrade benefits"),
  );

  final emailDesignHeaderBlock = BlockEntity(
    uuid: "launch-email-design-136",
    nodeType: "header4",
    content: AttributedText("Design Requirements"),
  );

  final emailDesignBlock1 = BlockEntity(
    uuid: "launch-email-design-137",
    nodeType: "list_item",
    content: AttributedText("Mobile-responsive templates"),
  );

  final emailDesignBlock2 = BlockEntity(
    uuid: "launch-email-design-138",
    nodeType: "list_item",
    content: AttributedText("Consistent brand colors and typography"),
  );

  final emailDesignBlock3 = BlockEntity(
    uuid: "launch-email-design-139",
    nodeType: "list_item",
    content: AttributedText("Clear CTAs with A/B testing variants"),
  );

  final emailDesignBlock4 = BlockEntity(
    uuid: "launch-email-design-140",
    nodeType: "list_item",
    content: AttributedText("Personalization tokens (name, company, trial status)"),
  );

  final emailTechHeaderBlock = BlockEntity(
    uuid: "launch-email-tech-141",
    nodeType: "header4",
    content: AttributedText("Technical Setup"),
  );

  final emailTechBlock1 = BlockEntity(
    uuid: "launch-email-tech-142",
    nodeType: "list_item",
    content: AttributedText("Mailchimp automation flows"),
  );

  final emailTechBlock2 = BlockEntity(
    uuid: "launch-email-tech-143",
    nodeType: "list_item",
    content: AttributedText("UTM tracking for attribution"),
  );

  final emailTechBlock3 = BlockEntity(
    uuid: "launch-email-tech-144",
    nodeType: "list_item",
    content: AttributedText("Deliverability optimization"),
  );

  @override
  void seed(SeedDataService s) {
    dbLog.finer('  <seeding project="${project.title.toPlainText()}">');
    final projectID = s.projectRepo.put(project);

    // Seed project description blocks
    final projectBlockIDs = s.blockRepo.putMany([
      projectHeaderBlock,
      projectDetailsBlock,
      strategyHeaderBlock,
      strategyBlock,
      featuresHeaderBlock,
      featuresListBlock1,
      featuresListBlock2,
      featuresListBlock3,
      featuresListBlock4,
      metricsHeaderBlock,
      metricsListBlock1,
      metricsListBlock2,
      metricsListBlock3,
      metricsListBlock4,
    ]);

    // Seed task description blocks
    final featureDevBlockIDs = s.blockRepo.putMany([
      featureDevHeaderBlock,
      featureDevStatusBlock,
      featureDevWorkHeaderBlock,
      featureDevWorkBlock1,
      featureDevWorkBlock2,
      featureDevWorkBlock3,
      featureDevBlockersBlock,
      featureDevMeetingsBlock,
    ]);

    final userAuthBlockIDs = s.blockRepo.putMany([
      userAuthHeaderBlock,
      userAuthFeaturesHeaderBlock,
      userAuthFeaturesBlock1,
      userAuthFeaturesBlock2,
      userAuthFeaturesBlock3,
      userAuthFeaturesBlock4,
      userAuthFeaturesBlock5,
      userAuthSecurityHeaderBlock,
      userAuthSecurityBlock1,
      userAuthSecurityBlock2,
      userAuthSecurityBlock3,
      userAuthSecurityBlock4,
      userAuthTestingBlock,
    ]);

    final analyticsBlockIDs = s.blockRepo.putMany([
      analyticsHeaderBlock,
      analyticsFeaturesHeaderBlock,
      analyticsFeaturesBlock1,
      analyticsFeaturesBlock2,
      analyticsFeaturesBlock3,
      analyticsFeaturesBlock4,
      analyticsFeaturesBlock5,
      analyticsTechHeaderBlock,
      analyticsTechBlock1,
      analyticsTechBlock2,
      analyticsTechBlock3,
      analyticsTechBlock4,
      analyticsPerfHeaderBlock,
      analyticsPerfBlock1,
      analyticsPerfBlock2,
      analyticsPerfBlock3,
    ]);

    final exportBlockIDs = s.blockRepo.putMany([
      exportHeaderBlock,
      exportFormatsHeaderBlock,
      exportFormatsBlock1,
      exportFormatsBlock2,
      exportFormatsBlock3,
      exportFormatsBlock4,
      exportFormatsBlock5,
      exportProgressHeaderBlock,
      exportProgressBlock1,
      exportProgressBlock2,
      exportProgressBlock3,
      exportNextHeaderBlock,
      exportNextBlock1,
      exportNextBlock2,
      exportNextBlock3,
    ]);

    final paymentBlockIDs = s.blockRepo.putMany([
      paymentHeaderBlock,
      paymentFeaturesHeaderBlock,
      paymentFeaturesBlock1,
      paymentFeaturesBlock2,
      paymentFeaturesBlock3,
      paymentFeaturesBlock4,
      paymentFeaturesBlock5,
      paymentImplHeaderBlock,
      paymentImplBlock1,
      paymentImplBlock2,
      paymentImplBlock3,
      paymentImplBlock4,
      paymentTestingHeaderBlock,
      paymentTestingBlock1,
      paymentTestingBlock2,
    ]);

    final marketingBlockIDs = s.blockRepo.putMany([
      marketingHeaderBlock,
      marketingStrategyHeaderBlock,
      marketingStrategyBlock1,
      marketingStrategyBlock2,
      marketingStrategyBlock3,
      marketingStrategyBlock4,
      marketingContentHeaderBlock,
      marketingContentBlock1,
      marketingContentBlock2,
      marketingContentBlock3,
      marketingContentBlock4,
      marketingContentBlock5,
      marketingTimelineHeaderBlock,
      marketingTimelineBlock1,
      marketingTimelineBlock2,
      marketingTimelineBlock3,
    ]);

    final productBlockIDs = s.blockRepo.putMany([
      productHeaderBlock,
      productTargetBlock,
      productOutlineHeaderBlock,
      productOutlineBlock1,
      productOutlineBlock2,
      productOutlineBlock3,
      productOutlineBlock4,
      productOutlineBlock5,
      productOutlineBlock6,
      productStatusHeaderBlock,
      productStatusBlock1,
      productStatusBlock2,
      productStatusBlock3,
      productStatusBlock4,
      productDistributionBlock,
    ]);

    final demoBlockIDs = s.blockRepo.putMany([
      demoHeaderBlock,
      demoConceptBlock,
      demoScriptHeaderBlock,
      demoScriptBlock1,
      demoScriptBlock2,
      demoScriptBlock3,
      demoScriptSubBlock1,
      demoScriptBlock4,
      demoScriptBlock5,
      demoProductionHeaderBlock,
      demoProductionBlock1,
      demoProductionBlock2,
      demoProductionBlock3,
      demoProductionBlock4,
    ]);

    final emailBlockIDs = s.blockRepo.putMany([
      emailHeaderBlock,
      emailSequenceHeaderBlock,
      emailSequenceBlock1,
      emailSequenceBlock2,
      emailSequenceBlock3,
      emailSequenceBlock4,
      emailSequenceBlock5,
      emailSequenceBlock6,
      emailSequenceBlock7,
      emailDesignHeaderBlock,
      emailDesignBlock1,
      emailDesignBlock2,
      emailDesignBlock3,
      emailDesignBlock4,
      emailTechHeaderBlock,
      emailTechBlock1,
      emailTechBlock2,
      emailTechBlock3,
    ]);

    // Feature dev task and subtasks
    final [
      featureDevID,
      userAuthID,
      analyticsDashboardID,
      exportID,
      paymentID,
    ] = s.taskRepo.putMany([
      featureDevTask,
      userAuthTask,
      analyticsDashboardTask,
      exportTask,
      paymentTask,
    ]);

    final savedFeatureDevTask = s.taskRepo.get(featureDevID)!;
    savedFeatureDevTask.children.addAll(
      s.blockRepo.getMany(featureDevBlockIDs).map((b) => b!.asThingEntity())
    );
    savedFeatureDevTask.children.addAll(
      s.taskRepo
          .getMany([userAuthID, analyticsDashboardID, exportID, paymentID])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedFeatureDevTask);

    // Add blocks to individual subtasks
    final savedUserAuthTask = s.taskRepo.get(userAuthID)!;
    savedUserAuthTask.children.addAll(
      s.blockRepo.getMany(userAuthBlockIDs).map((b) => b!.asThingEntity())
    );
    s.taskRepo.put(savedUserAuthTask);

    final savedAnalyticsTask = s.taskRepo.get(analyticsDashboardID)!;
    savedAnalyticsTask.children.addAll(
      s.blockRepo.getMany(analyticsBlockIDs).map((b) => b!.asThingEntity())
    );
    s.taskRepo.put(savedAnalyticsTask);

    final savedExportTask = s.taskRepo.get(exportID)!;
    savedExportTask.children.addAll(
      s.blockRepo.getMany(exportBlockIDs).map((b) => b!.asThingEntity())
    );
    s.taskRepo.put(savedExportTask);

    final savedPaymentTask = s.taskRepo.get(paymentID)!;
    savedPaymentTask.children.addAll(
      s.blockRepo.getMany(paymentBlockIDs).map((b) => b!.asThingEntity())
    );
    s.taskRepo.put(savedPaymentTask);

    // Marketing task and subtasks
    final [marketingID, productID, demoID, emailID] = s.taskRepo.putMany([
      marketingTask,
      productTask,
      demoTask,
      emailTask,
    ]);

    final savedMarketingTask = s.taskRepo.get(marketingID)!;
    savedMarketingTask.children.addAll(
      s.blockRepo.getMany(marketingBlockIDs).map((b) => b!.asThingEntity())
    );
    savedMarketingTask.children.addAll(
      s.taskRepo
          .getMany([productID, demoID, emailID])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedMarketingTask);

    // Add blocks to marketing subtasks
    final savedProductTask = s.taskRepo.get(productID)!;
    savedProductTask.children.addAll(
      s.blockRepo.getMany(productBlockIDs).map((b) => b!.asThingEntity())
    );
    s.taskRepo.put(savedProductTask);

    final savedDemoTask = s.taskRepo.get(demoID)!;
    savedDemoTask.children.addAll(
      s.blockRepo.getMany(demoBlockIDs).map((b) => b!.asThingEntity())
    );
    s.taskRepo.put(savedDemoTask);

    final savedEmailTask = s.taskRepo.get(emailID)!;
    savedEmailTask.children.addAll(
      s.blockRepo.getMany(emailBlockIDs).map((b) => b!.asThingEntity())
    );
    s.taskRepo.put(savedEmailTask);

    final savedProject = s.projectRepo.get(projectID)!;
    savedProject.children.addAll(
      s.blockRepo.getMany(projectBlockIDs).map((b) => b!.asThingEntity())
    );
    savedProject.children.addAll(
      s.taskRepo
          .getMany([featureDevID, marketingID])
          .map((t) => t!.asThingEntity()),
    );
    s.projectRepo.put(savedProject);
  }
}
