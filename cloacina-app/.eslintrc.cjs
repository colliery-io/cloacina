module.exports = {
  env: {
    browser: true,
    es2021: true,
    node: true
  },
  extends: [
    'eslint:recommended'
  ],
  plugins: ['import'],
  parserOptions: {
    ecmaVersion: 'latest',
    sourceType: 'module'
  },
  rules: {
    // Dead code detection rules
    'no-unused-vars': ['error', {
      'argsIgnorePattern': '^_',
      'varsIgnorePattern': '^_'
    }],
    'no-unreachable': 'error',
    'no-unreachable-loop': 'error',

    // Import/export dead code detection
    'import/no-unused-modules': ['error', {
      unusedExports: true,
      missingExports: true
    }],
    'import/no-duplicates': 'error',

    // General code quality
    'no-console': 'warn',
    'prefer-const': 'error',
    'no-var': 'error'
  },
  globals: {
    // Tauri globals
    '__TAURI__': 'readonly',
    // D3 globals (since you're using CDN)
    'd3': 'readonly',
    'dagreD3': 'readonly'
  }
};
