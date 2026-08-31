export default {
  output: 'standalone',
  serverExternalPackages: ['@fuzzy-parser/node'],
  outputFileTracingIncludes: {
    '/api/parse': ['./node_modules/@fuzzy-parser/node/dist/**/*'],
  },
};
