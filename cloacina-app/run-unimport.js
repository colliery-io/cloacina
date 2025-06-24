#!/usr/bin/env node

// Run unimport programmatically since CLI might not be available
import { scanFilesFromGlob } from 'unimport';

async function runUnimport() {
  try {
    console.log('🔍 Running unimport analysis...\n');

    const result = await scanFilesFromGlob(['src/**/*.js'], {
      cwd: process.cwd(),
    });

    console.log('📊 Unimport Results:');
    console.log(`- Files scanned: ${result.files?.length || 0}`);
    console.log(`- Imports found: ${result.imports?.length || 0}`);
    console.log(`- Exports found: ${result.exports?.length || 0}`);

    if (result.unused && result.unused.length > 0) {
      console.log('\n⚠️  Unused imports:');
      result.unused.forEach(item => {
        console.log(`- ${item.source}: ${item.name}`);
      });
    } else {
      console.log('\n✅ No unused imports detected');
    }

  } catch (error) {
    console.error('❌ Error running unimport:', error.message);
    console.log('\n💡 Fallback: Use our custom dead code script instead');
    console.log('Run: npm run dead-code');
  }
}

runUnimport();
