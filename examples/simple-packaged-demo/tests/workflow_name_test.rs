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

/*
 *  Test to verify the actual workflow name being created
 */

use simple_packaged_demo::data_processing;
use std::ffi::CString;

#[test]
fn test_actual_workflow_name() {
    let tenant_id = CString::new("test_tenant").unwrap();
    let workflow_id = CString::new("test_workflow").unwrap();

    let workflow_ptr = unsafe {
        data_processing::cloacina_create_workflow(tenant_id.as_ptr(), workflow_id.as_ptr())
    };

    assert!(!workflow_ptr.is_null());

    let workflow = unsafe { Box::from_raw(workflow_ptr as *mut cloacina::workflow::Workflow) };

    println!("Actual workflow name: '{}'", workflow.name());
    println!("Actual workflow package: '{}'", workflow.package());
    println!("Actual workflow tenant: '{}'", workflow.tenant());

    // Test what the workflow name actually is
    assert_eq!(
        workflow.name(),
        "data_processing",
        "Workflow name should be 'data_processing'"
    );
    assert_eq!(
        workflow.package(),
        "simple_demo",
        "Package should be 'simple_demo'"
    );
}
