#!/usr/bin/env node

/**
 * Analyze the main.js vs main-modular.js files to find duplicate/dead code
 */

const fs = require('fs');

console.log('🔍 Analyzing main.js vs main-modular.js for dead code...\n');

// Check which main file is actually being used
const indexHtml = fs.readFileSync('index.html', 'utf8');
const usesModular = indexHtml.includes('main-modular.js');
const usesOldMain = indexHtml.includes('main.js') && !indexHtml.includes('main-modular.js');

console.log('📄 Current Usage:');
console.log(`- index.html uses: ${usesModular ? 'main-modular.js' : 'main.js'}`);

if (usesModular) {
  const mainSize = fs.statSync('main.js').size;
  console.log(`- main.js is ${Math.round(mainSize/1024)}KB and appears to be DEAD CODE`);
  console.log('- Recommendation: main.js can likely be deleted\n');
} else {
  console.log('- main-modular.js appears to be unused\n');
}

// Check for other potentially dead files
const potentialDeadFiles = [
  'new-main.js',
  'dagre-visualization.js'
];

console.log('🗑️  Other Potentially Dead Files:');
for (const file of potentialDeadFiles) {
  try {
    const stat = fs.statSync(file);
    const size = Math.round(stat.size / 1024);
    console.log(`- ${file} (${size}KB) - check if referenced anywhere`);
  } catch (e) {
    console.log(`- ${file} - not found`);
  }
}

// Check constants usage
console.log('\n📊 Constants Usage Analysis:');
const constants = [
  'RUNNER_STATUS',
  'EXECUTION_STATUS',
  'BUILD_PROFILES',
  'FILE_FILTERS',
  'LOG_LEVELS'
];

const allJsFiles = [
  'main-modular.js',
  'modules/app/initialization.js',
  'modules/app/navigation.js',
  'modules/app/settings.js',
  'modules/runners/management.js',
  'modules/packages/build.js',
  'modules/packages/debug.js',
  'modules/packages/inspect.js',
  'utils/api-client.js',
  'utils/file-dialogs.js',
  'utils/ui-helpers.js'
];

for (const constant of constants) {
  let used = false;
  for (const file of allJsFiles) {
    try {
      const content = fs.readFileSync(file, 'utf8');
      if (content.includes(constant)) {
        used = true;
        break;
      }
    } catch (e) {
      // File doesn't exist, skip
    }
  }

  console.log(`- ${constant}: ${used ? '✅ USED' : '❌ UNUSED'}`);
}

console.log('\n💡 Recommendations:');
if (usesModular) {
  console.log('1. DELETE main.js (67KB) - it\'s been replaced by modular structure');
}
console.log('2. Clean up unused constants from app-constants.js');
console.log('3. Consider deleting dagre-visualization.js if it\'s not used');
console.log('4. Review UiHelpers class - might have unused methods');
