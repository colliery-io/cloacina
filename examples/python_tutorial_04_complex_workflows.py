#!/usr/bin/env python3
"""
Cloacina Python Tutorial 04: Complex Workflows

This example demonstrates sophisticated workflows with advanced dependency
patterns including diamond patterns, fan-out/fan-in architectures,
multi-level chains, and complex mixed patterns.

Learning objectives:
- Design diamond dependency patterns for fork-join operations
- Implement fan-out patterns for parallel task execution
- Create fan-in patterns for result aggregation
- Build multi-level dependency chains
- Combine patterns for complex workflow architectures
- Optimize workflow performance through smart dependency design

Usage:
    python python_tutorial_04_complex_workflows.py

Prerequisites:
    pip install cloaca[sqlite]
"""

import sys
import cloaca
import random
import time
from datetime import datetime

# Diamond Pattern: Fork-Join Processing
@cloaca.task(id="prepare_dataset")
def prepare_dataset(context):
    """Prepare initial dataset for parallel processing."""
    print("Preparing dataset...")

    # Generate sample dataset
    dataset = {
        "customers": [
            {"id": i, "name": f"Customer_{i}", "segment": random.choice(["premium", "standard", "basic"]), "value": random.randint(100, 10000)}
            for i in range(1, 101)
        ],
        "prepared_at": datetime.now().isoformat(),
        "total_records": 100
    }

    context.set("dataset", dataset)
    context.set("preparation_complete", True)

    print(f"Dataset prepared with {len(dataset['customers'])} customers")
    return context

# Parallel Processing Tasks (Fork)
@cloaca.task(id="analyze_segments", dependencies=["prepare_dataset"])
def analyze_segments(context):
    """Analyze customer segments in parallel."""
    print("Analyzing customer segments...")

    dataset = context.get("dataset")
    customers = dataset["customers"]

    # Segment analysis
    segments = {}
    for customer in customers:
        segment = customer["segment"]
        if segment not in segments:
            segments[segment] = {"count": 0, "total_value": 0, "customers": []}

        segments[segment]["count"] += 1
        segments[segment]["total_value"] += customer["value"]
        segments[segment]["customers"].append(customer["id"])

    # Calculate averages
    for segment, data in segments.items():
        data["average_value"] = data["total_value"] / data["count"]

    segment_analysis = {
        "segments": segments,
        "analyzed_at": datetime.now().isoformat(),
        "total_segments": len(segments)
    }

    context.set("segment_analysis", segment_analysis)
    print(f"Segment analysis complete: {len(segments)} segments found")
    return context

@cloaca.task(id="calculate_metrics", dependencies=["prepare_dataset"])
def calculate_metrics(context):
    """Calculate key metrics in parallel."""
    print("Calculating metrics...")

    dataset = context.get("dataset")
    customers = dataset["customers"]

    # Calculate various metrics
    values = [c["value"] for c in customers]

    metrics = {
        "total_customers": len(customers),
        "total_value": sum(values),
        "average_value": sum(values) / len(values),
        "min_value": min(values),
        "max_value": max(values),
        "median_value": sorted(values)[len(values) // 2],
        "value_distribution": {
            "low": len([v for v in values if v < 1000]),
            "medium": len([v for v in values if 1000 <= v < 5000]),
            "high": len([v for v in values if v >= 5000])
        },
        "calculated_at": datetime.now().isoformat()
    }

    context.set("metrics", metrics)
    print(f"Metrics calculated - Average value: ${metrics['average_value']:.2f}")
    return context

# Join point
@cloaca.task(id="combine_analysis", dependencies=["analyze_segments", "calculate_metrics"])
def combine_analysis(context):
    """Combine parallel analysis results."""
    print("Combining analysis results...")

    segment_analysis = context.get("segment_analysis")
    metrics = context.get("metrics")

    # Create comprehensive analysis
    combined_analysis = {
        "overview": {
            "total_customers": metrics["total_customers"],
            "total_value": metrics["total_value"],
            "average_value": metrics["average_value"],
            "segments_identified": segment_analysis["total_segments"]
        },
        "segment_breakdown": segment_analysis["segments"],
        "value_metrics": {
            "min": metrics["min_value"],
            "max": metrics["max_value"],
            "median": metrics["median_value"],
            "distribution": metrics["value_distribution"]
        },
        "insights": [],
        "combined_at": datetime.now().isoformat()
    }

    # Generate insights from combined data
    insights = []

    # Segment insights
    for segment, data in segment_analysis["segments"].items():
        percentage = (data["count"] / metrics["total_customers"]) * 100
        insights.append(f"{segment.title()} segment: {data['count']} customers ({percentage:.1f}%), avg value ${data['average_value']:.2f}")

    # Value distribution insights
    dist = metrics["value_distribution"]
    insights.append(f"Value distribution: {dist['low']} low-value, {dist['medium']} medium-value, {dist['high']} high-value customers")

    combined_analysis["insights"] = insights

    context.set("combined_analysis", combined_analysis)
    print(f"Analysis combined with {len(insights)} insights generated")
    return context

# Fan-Out Pattern: Parallel Processing
@cloaca.task(id="initiate_campaigns", dependencies=["combine_analysis"])
def initiate_campaigns(context):
    """Initiate multiple marketing campaigns based on analysis."""
    print("Initiating marketing campaigns...")

    combined_analysis = context.get("combined_analysis")
    segments = combined_analysis["segment_breakdown"]

    # Create campaign specifications for each segment
    campaigns = {}
    campaign_id = 1

    for segment, data in segments.items():
        campaign = {
            "campaign_id": f"CAMP_{campaign_id:03d}",
            "target_segment": segment,
            "target_customer_count": data["count"],
            "average_customer_value": data["average_value"],
            "campaign_type": "email" if segment == "basic" else "premium_email" if segment == "standard" else "personal_outreach",
            "budget": data["total_value"] * 0.05,  # 5% of segment value
            "priority": "high" if segment == "premium" else "medium" if segment == "standard" else "low",
            "initiated_at": datetime.now().isoformat()
        }
        campaigns[f"campaign_{campaign_id}"] = campaign
        campaign_id += 1

    context.set("campaigns", campaigns)
    context.set("campaign_initiation_complete", True)

    print(f"Initiated {len(campaigns)} campaigns")
    return context

# Parallel Campaign Execution Tasks
@cloaca.task(id="execute_email_campaigns", dependencies=["initiate_campaigns"])
def execute_email_campaigns(context):
    """Execute email-based campaigns."""
    print("Executing email campaigns...")

    campaigns = context.get("campaigns", {})
    email_campaigns = {k: v for k, v in campaigns.items() if v["campaign_type"] in ["email", "premium_email"]}

    execution_results = []

    for campaign_key, campaign in email_campaigns.items():
        # Simulate campaign execution
        execution_time = random.uniform(1, 3)  # 1-3 seconds
        time.sleep(execution_time)

        success_rate = random.uniform(0.7, 0.95)  # 70-95% success rate
        successful_sends = int(campaign["target_customer_count"] * success_rate)

        result = {
            "campaign_id": campaign["campaign_id"],
            "campaign_type": campaign["campaign_type"],
            "target_count": campaign["target_customer_count"],
            "successful_sends": successful_sends,
            "success_rate": success_rate,
            "execution_time": execution_time,
            "cost": successful_sends * (2.0 if campaign["campaign_type"] == "premium_email" else 0.5),
            "executed_at": datetime.now().isoformat()
        }

        execution_results.append(result)
        print(f"  Email campaign {campaign['campaign_id']}: {successful_sends}/{campaign['target_customer_count']} sent ({success_rate:.1%})")

    context.set("email_campaign_results", execution_results)
    print(f"Email campaigns completed: {len(execution_results)} campaigns executed")
    return context

@cloaca.task(id="execute_outreach_campaigns", dependencies=["initiate_campaigns"])
def execute_outreach_campaigns(context):
    """Execute personal outreach campaigns."""
    print("Executing personal outreach campaigns...")

    campaigns = context.get("campaigns", {})
    outreach_campaigns = {k: v for k, v in campaigns.items() if v["campaign_type"] == "personal_outreach"}

    execution_results = []

    for campaign_key, campaign in outreach_campaigns.items():
        # Simulate personal outreach (slower but higher conversion)
        execution_time = random.uniform(2, 5)  # 2-5 seconds
        time.sleep(execution_time)

        contact_rate = random.uniform(0.4, 0.7)  # 40-70% contact rate
        contacts_made = int(campaign["target_customer_count"] * contact_rate)

        conversion_rate = random.uniform(0.15, 0.35)  # 15-35% conversion rate
        conversions = int(contacts_made * conversion_rate)

        result = {
            "campaign_id": campaign["campaign_id"],
            "campaign_type": campaign["campaign_type"],
            "target_count": campaign["target_customer_count"],
            "contacts_made": contacts_made,
            "conversions": conversions,
            "contact_rate": contact_rate,
            "conversion_rate": conversion_rate,
            "execution_time": execution_time,
            "cost": contacts_made * 25.0,  # $25 per contact
            "revenue": conversions * campaign["average_customer_value"] * 0.1,  # 10% of customer value
            "executed_at": datetime.now().isoformat()
        }

        execution_results.append(result)
        print(f"  Outreach campaign {campaign['campaign_id']}: {contacts_made} contacts, {conversions} conversions")

    context.set("outreach_campaign_results", execution_results)
    print(f"Outreach campaigns completed: {len(execution_results)} campaigns executed")
    return context

# Fan-In Pattern: Aggregate Results
@cloaca.task(id="aggregate_campaign_results", dependencies=["execute_email_campaigns", "execute_outreach_campaigns"])
def aggregate_campaign_results(context):
    """Aggregate all campaign results."""
    print("Aggregating campaign results...")

    email_results = context.get("email_campaign_results", [])
    outreach_results = context.get("outreach_campaign_results", [])

    all_results = email_results + outreach_results

    # Calculate aggregate metrics
    total_targets = sum(r["target_count"] for r in all_results)
    total_cost = sum(r["cost"] for r in all_results)
    total_revenue = sum(r.get("revenue", 0) for r in all_results)

    # Email-specific metrics
    email_sends = sum(r["successful_sends"] for r in email_results)
    email_cost = sum(r["cost"] for r in email_results)

    # Outreach-specific metrics
    total_contacts = sum(r["contacts_made"] for r in outreach_results)
    total_conversions = sum(r["conversions"] for r in outreach_results)
    outreach_cost = sum(r["cost"] for r in outreach_results)
    outreach_revenue = sum(r.get("revenue", 0) for r in outreach_results)

    aggregated_results = {
        "campaign_summary": {
            "total_campaigns": len(all_results),
            "email_campaigns": len(email_results),
            "outreach_campaigns": len(outreach_results),
            "total_targets": total_targets,
            "total_cost": total_cost,
            "total_revenue": total_revenue,
            "roi": (total_revenue - total_cost) / total_cost if total_cost > 0 else 0
        },
        "email_performance": {
            "campaigns": len(email_results),
            "total_sends": email_sends,
            "total_cost": email_cost,
            "cost_per_send": email_cost / email_sends if email_sends > 0 else 0
        },
        "outreach_performance": {
            "campaigns": len(outreach_results),
            "total_contacts": total_contacts,
            "total_conversions": total_conversions,
            "conversion_rate": total_conversions / total_contacts if total_contacts > 0 else 0,
            "total_cost": outreach_cost,
            "total_revenue": outreach_revenue,
            "roi": (outreach_revenue - outreach_cost) / outreach_cost if outreach_cost > 0 else 0
        },
        "detailed_results": all_results,
        "aggregated_at": datetime.now().isoformat()
    }

    context.set("aggregated_results", aggregated_results)

    print("Results aggregated:")
    print(f"  Total campaigns: {len(all_results)}")
    print(f"  Total cost: ${total_cost:.2f}")
    print(f"  Total revenue: ${total_revenue:.2f}")
    print(f"  Overall ROI: {aggregated_results['campaign_summary']['roi']:.1%}")

    return context

# Multi-Level Chain: Sequential Analysis and Optimization
@cloaca.task(id="analyze_performance", dependencies=["aggregate_campaign_results"])
def analyze_performance(context):
    """Analyze campaign performance for optimization insights."""
    print("Analyzing performance...")

    aggregated_results = context.get("aggregated_results")
    detailed_results = aggregated_results["detailed_results"]

    # Performance analysis
    performance_analysis = {
        "top_performing_campaigns": [],
        "underperforming_campaigns": [],
        "optimization_opportunities": [],
        "cost_efficiency": {},
        "analyzed_at": datetime.now().isoformat()
    }

    # Sort campaigns by ROI for outreach, by cost efficiency for email
    email_campaigns = [r for r in detailed_results if r["campaign_type"] in ["email", "premium_email"]]
    outreach_campaigns = [r for r in detailed_results if r["campaign_type"] == "personal_outreach"]

    # Analyze email campaigns by cost efficiency
    for campaign in email_campaigns:
        efficiency = campaign["successful_sends"] / campaign["cost"] if campaign["cost"] > 0 else 0
        campaign["efficiency_metric"] = efficiency

    # Analyze outreach campaigns by ROI
    for campaign in outreach_campaigns:
        roi = (campaign.get("revenue", 0) - campaign["cost"]) / campaign["cost"] if campaign["cost"] > 0 else 0
        campaign["roi"] = roi

    # Identify top performers
    if email_campaigns:
        top_email = max(email_campaigns, key=lambda x: x["efficiency_metric"])
        performance_analysis["top_performing_campaigns"].append({
            "campaign_id": top_email["campaign_id"],
            "type": "email",
            "metric": "efficiency",
            "value": top_email["efficiency_metric"]
        })

    if outreach_campaigns:
        top_outreach = max(outreach_campaigns, key=lambda x: x["roi"])
        performance_analysis["top_performing_campaigns"].append({
            "campaign_id": top_outreach["campaign_id"],
            "type": "outreach",
            "metric": "roi",
            "value": top_outreach["roi"]
        })

    # Generate optimization recommendations
    optimizations = []

    overall_roi = aggregated_results["campaign_summary"]["roi"]
    if overall_roi < 0.1:  # Less than 10% ROI
        optimizations.append("Overall ROI is low - consider reducing campaign spend or improving targeting")

    outreach_roi = aggregated_results["outreach_performance"]["roi"]
    email_efficiency = aggregated_results["email_performance"]["cost_per_send"]

    if outreach_roi > 0.2:  # Good outreach ROI
        optimizations.append("Personal outreach showing strong ROI - consider expanding outreach budget")

    if email_efficiency < 1.0:  # Good email efficiency
        optimizations.append("Email campaigns are cost-efficient - consider increasing email volume")

    performance_analysis["optimization_opportunities"] = optimizations

    context.set("performance_analysis", performance_analysis)

    print(f"Performance analysis completed with {len(optimizations)} optimization opportunities")
    return context

@cloaca.task(id="generate_recommendations", dependencies=["analyze_performance"])
def generate_recommendations(context):
    """Generate final recommendations based on complete analysis."""
    print("Generating final recommendations...")

    # Gather all analysis data
    combined_analysis = context.get("combined_analysis")
    aggregated_results = context.get("aggregated_results")

    # Create comprehensive recommendations
    final_recommendations = {
        "executive_summary": {
            "total_customers_analyzed": combined_analysis["overview"]["total_customers"],
            "campaigns_executed": aggregated_results["campaign_summary"]["total_campaigns"],
            "total_investment": aggregated_results["campaign_summary"]["total_cost"],
            "total_return": aggregated_results["campaign_summary"]["total_revenue"],
            "overall_roi": aggregated_results["campaign_summary"]["roi"]
        },
        "strategic_recommendations": [],
        "tactical_recommendations": [],
        "budget_recommendations": {},
        "next_steps": [],
        "generated_at": datetime.now().isoformat()
    }

    # Strategic recommendations
    strategic = []

    premium_segment = combined_analysis["segment_breakdown"].get("premium", {})
    if premium_segment.get("count", 0) > 0:
        premium_avg = premium_segment["average_value"]
        strategic.append(f"Premium segment ({premium_segment['count']} customers) has high average value (${premium_avg:.2f}) - prioritize retention programs")

    overall_roi = aggregated_results["campaign_summary"]["roi"]
    if overall_roi > 0.15:
        strategic.append("Campaign performance exceeds targets - scale successful campaign types")
    elif overall_roi < 0.05:
        strategic.append("Campaign performance below expectations - review targeting and messaging strategy")

    # Tactical recommendations
    tactical = []

    outreach_roi = aggregated_results["outreach_performance"]["roi"]
    email_efficiency = aggregated_results["email_performance"]["cost_per_send"]

    if outreach_roi > 0.2:
        tactical.append("Personal outreach shows strong ROI - increase outreach team capacity")

    if email_efficiency < 2.0:
        tactical.append("Email campaigns are highly cost-effective - expand email automation")

    conversion_rate = aggregated_results["outreach_performance"]["conversion_rate"]
    if conversion_rate > 0.25:
        tactical.append("Outreach conversion rate is excellent - document and replicate successful scripts")

    # Budget recommendations
    total_budget = aggregated_results["campaign_summary"]["total_cost"]
    budget_recs = {
        "current_allocation": {
            "email": aggregated_results["email_performance"]["total_cost"],
            "outreach": aggregated_results["outreach_performance"]["total_cost"]
        },
        "recommended_allocation": {},
        "budget_increase_justification": ""
    }

    if outreach_roi > email_efficiency / 10:  # Outreach ROI better than email efficiency
        budget_recs["recommended_allocation"] = {
            "email": total_budget * 0.3,
            "outreach": total_budget * 0.7
        }
        budget_recs["budget_increase_justification"] = "Increase outreach allocation due to higher ROI"
    else:
        budget_recs["recommended_allocation"] = {
            "email": total_budget * 0.6,
            "outreach": total_budget * 0.4
        }
        budget_recs["budget_increase_justification"] = "Maintain email focus due to cost efficiency"

    # Next steps
    next_steps = [
        "Implement A/B testing for top-performing campaign types",
        "Develop segment-specific messaging strategies",
        "Set up automated performance monitoring",
        "Plan quarterly campaign optimization reviews"
    ]

    final_recommendations.update({
        "strategic_recommendations": strategic,
        "tactical_recommendations": tactical,
        "budget_recommendations": budget_recs,
        "next_steps": next_steps
    })

    context.set("final_recommendations", final_recommendations)

    print("Final recommendations generated:")
    print(f"  Strategic recommendations: {len(strategic)}")
    print(f"  Tactical recommendations: {len(tactical)}")
    print(f"  Next steps defined: {len(next_steps)}")

    return context

# Create the complex workflow
def create_complex_workflow():
    """Build the complex workflow with multiple patterns."""
    builder = cloaca.WorkflowBuilder("complex_workflow")
    builder.description("Complex workflow demonstrating diamond, fan-out, fan-in, and multi-level chain patterns")

    # Add all tasks
    builder.add_task("prepare_dataset")

    # Diamond pattern (fork)
    builder.add_task("analyze_segments")
    builder.add_task("calculate_metrics")

    # Diamond pattern (join)
    builder.add_task("combine_analysis")

    # Fan-out pattern initiation
    builder.add_task("initiate_campaigns")

    # Fan-out pattern (parallel execution)
    builder.add_task("execute_email_campaigns")
    builder.add_task("execute_outreach_campaigns")

    # Fan-in pattern (aggregation)
    builder.add_task("aggregate_campaign_results")

    # Multi-level chain
    builder.add_task("analyze_performance")
    builder.add_task("generate_recommendations")

    return builder.build()

# Register the workflow
cloaca.register_workflow_constructor("complex_workflow", create_complex_workflow)

# Main execution
if __name__ == "__main__":
    print("=== Cloacina Python Tutorial 04: Complex Workflows ===")
    print()
    print("This tutorial demonstrates:")
    print("- Diamond pattern (fork-join) for parallel analysis")
    print("- Fan-out pattern for parallel campaign execution")
    print("- Fan-in pattern for result aggregation")
    print("- Multi-level chains for sequential optimization")
    print("- Mixed patterns in a single complex workflow")
    print()

    # Create runner
    runner = cloaca.DefaultRunner("sqlite://:memory:")

    # Create initial context
    context = cloaca.Context({
        "tutorial": "04",
        "workflow_type": "complex_patterns",
        "simulation_mode": True
    })

    print("Executing complex workflow...")
    print("This workflow includes multiple parallel processing patterns")
    print()

    # Execute the workflow
    result = runner.execute("complex_workflow", context)

    # Display results
    print(f"\nWorkflow Status: {result.status}")

    if result.status == "Completed":
        print("Success! Complex workflow completed.")

        # Access the final recommendations
        final_context = result.final_context
        recommendations = final_context.get("final_recommendations")

        if recommendations:
            print("\n=== Workflow Results ===")
            summary = recommendations["executive_summary"]
            print(f"Customers analyzed: {summary['total_customers_analyzed']}")
            print(f"Campaigns executed: {summary['campaigns_executed']}")
            print(f"Total investment: ${summary['total_investment']:.2f}")
            print(f"Total return: ${summary['total_return']:.2f}")
            print(f"Overall ROI: {summary['overall_roi']:.1%}")

            print(f"\nStrategic recommendations: {len(recommendations['strategic_recommendations'])}")
            for rec in recommendations['strategic_recommendations']:
                print(f"  • {rec}")

            print(f"\nTactical recommendations: {len(recommendations['tactical_recommendations'])}")
            for rec in recommendations['tactical_recommendations']:
                print(f"  • {rec}")

        performance_analysis = final_context.get("performance_analysis")  # noqa: F841

    else:
        print(f"Workflow failed with status: {result.status}")
        if hasattr(result, 'error'):
            print(f"Error: {result.error}")

        # Clean up before exiting
        print("\nCleaning up...")
        runner.shutdown()
        sys.exit(1)

    # Cleanup
    print("\nCleaning up...")
    runner.shutdown()
    print("Tutorial 04 completed!")
    print()
    print("Key patterns demonstrated:")
    print("- Diamond pattern: prepare_dataset → [analyze_segments + calculate_metrics] → combine_analysis")
    print("- Fan-out pattern: initiate_campaigns → [execute_email_campaigns + execute_outreach_campaigns]")
    print("- Fan-in pattern: [email_results + outreach_results] → aggregate_campaign_results")
    print("- Multi-level chain: aggregate → analyze_performance → generate_recommendations")
    print()
    print("Next steps:")
    print("- Try python_tutorial_05_multi_tenancy.py for PostgreSQL multi-tenancy")
    print("- Experiment with different dependency patterns")
    print("- Design your own complex workflow architectures")
