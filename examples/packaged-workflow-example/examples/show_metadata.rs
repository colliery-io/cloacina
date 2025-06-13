/*
 *  Copyright 2025 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use packaged_workflow_example::{analytics_workflow, marketing_workflow};

fn main() {
    println!("=== Packaged Workflow Metadata ===\n");

    // Show analytics pipeline metadata
    let analytics_meta = analytics_workflow::get_package_metadata();
    println!("📊 Analytics Pipeline Package:");
    println!("   Package: {}", analytics_meta.package);
    println!("   Version: {}", analytics_meta.version);
    println!("   Author: {}", analytics_meta.author);
    println!("   Description: {}", analytics_meta.description);
    println!("   Fingerprint: {}", analytics_meta.fingerprint);
    println!();

    // Show marketing campaign metadata
    let marketing_meta = marketing_workflow::get_package_metadata();
    println!("📈 Marketing Campaign Package:");
    println!("   Package: {}", marketing_meta.package);
    println!("   Version: {}", marketing_meta.version);
    println!("   Author: {}", marketing_meta.author);
    println!("   Description: {}", marketing_meta.description);
    println!("   Fingerprint: {}", marketing_meta.fingerprint);
    println!();

    println!("=== Task Registration Simulation ===\n");

    // Show task registration (in real scenarios these would register in global registry)
    println!("📋 Registering analytics tasks for tenant 'acme_corp' in workflow 'data_pipeline':");
    analytics_workflow::register_package_tasks("acme_corp", "data_pipeline");
    println!(
        "   ✅ Tasks registered under namespace: acme_corp::analytics_pipeline::data_pipeline::*"
    );
    println!();

    println!("📋 Registering marketing tasks for tenant 'retail_co' in workflow 'campaign_mgmt':");
    marketing_workflow::register_package_tasks("retail_co", "campaign_mgmt");
    println!(
        "   ✅ Tasks registered under namespace: retail_co::marketing_campaigns::campaign_mgmt::*"
    );
    println!();

    println!("🏗️  ABI Functions Available:");
    println!("   - register_tasks_abi_analytics_pipeline()");
    println!("   - get_package_metadata_abi_analytics_pipeline()");
    println!("   - register_tasks_abi_marketing_campaigns()");
    println!("   - get_package_metadata_abi_marketing_campaigns()");
    println!();

    println!("🔧 These functions enable dynamic loading when compiled as .so files");
}
