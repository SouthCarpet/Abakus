import js from '@eslint/js'
import tseslint from 'typescript-eslint'
export default tseslint.config(js.configs.recommended, ...tseslint.configs.recommended, { rules: { complexity: ['error', 12], 'max-depth': ['error', 3] } })
