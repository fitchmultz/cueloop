// Package project defines Ralph project types and normalization helpers.
package project

import "strings"

// Type describes the repo focus that drives prompt defaults.
type Type string

const (
	TypeCode Type = "code"
	TypeDocs Type = "docs"
)

// DefaultType returns the default project type.
func DefaultType() Type {
	return TypeCode
}

// NormalizeType cleans user input into a canonical project type string.
func NormalizeType(value string) Type {
	trimmed := strings.ToLower(strings.TrimSpace(value))
	return Type(trimmed)
}

// ValidType reports whether the project type is supported.
func ValidType(value Type) bool {
	switch value {
	case TypeCode, TypeDocs:
		return true
	default:
		return false
	}
}

// AllowedTypes returns the supported project type values.
func AllowedTypes() []Type {
	return []Type{TypeCode, TypeDocs}
}
