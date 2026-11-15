package rrd

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"regexp"
	"strings"
	"time"
)

var (
	correlationPattern = regexp.MustCompile(`^[A-Za-z0-9._:-]+$`)
	canonicalPattern   = regexp.MustCompile(`^[a-z0-9][a-z0-9._-]*$`)
	pathParameter      = regexp.MustCompile(`\{([a-z]+)\}`)
	resourceKinds      = map[string]struct{}{
		"organization": {}, "engram": {}, "project": {}, "instance": {}, "node": {},
		"shard": {}, "collection": {}, "table": {}, "record": {}, "transaction": {},
		"snapshot": {}, "backup": {}, "operation": {},
	}
)

// Endpoint is the current generated operation descriptor projection.
type Endpoint struct {
	Method         string
	Path           string
	Authentication string
	Mutation       bool
}

// ResourceSegment identifies one component of an RRD resource path.
type ResourceSegment struct {
	Kind string `json:"kind"`
	ID   string `json:"id"`
}

// RequestOptions contains the current common operation metadata.
// Operation-specific generated request models remain gated to H-04.
type RequestOptions struct {
	RequestID      string
	OperationID    string
	IdempotencyKey string
	Deadline       time.Time
	PathParameters map[string]string
	Resource       []ResourceSegment
	Session        *Session
	APIKey         *APIKeyCredentials
}

type requestEnvelopeContext struct {
	RequestID      string `json:"request_id"`
	OperationID    string `json:"operation_id"`
	IdempotencyKey string `json:"idempotency_key,omitempty"`
	DeadlineUnixMS int64  `json:"deadline_unix_ms,omitempty"`
}

func (c *Client) executeOperation(
	ctx context.Context,
	operation OperationID,
	payload any,
	options RequestOptions,
) (map[string]any, error) {
	descriptor, ok := endpoints[operation]
	if !ok {
		return nil, fmt.Errorf("unknown RRD operation %q", operation)
	}
	requestContext, err := makeContext(options, descriptor)
	if err != nil {
		return nil, err
	}
	path, err := resolvePath(descriptor.Path, options.PathParameters)
	if err != nil {
		return nil, err
	}
	target := c.baseURL.ResolveReference(&url.URL{Path: strings.TrimPrefix(path, "/")})
	headers := http.Header{"Accept": []string{"application/json"}}
	switch descriptor.Authentication {
	case "api_key":
		if options.APIKey == nil || options.APIKey.Credential == "" {
			return nil, fmt.Errorf("%s requires API-key authentication", operation)
		}
		principal, err := canonical(options.APIKey.PrincipalID, "principal")
		if err != nil {
			return nil, err
		}
		headers.Set("X-RRD-Principal", principal)
		headers.Set("Authorization", "ApiKey "+options.APIKey.Credential)
	case "session_bearer":
		if options.Session == nil || options.Session.Lease.SessionID == "" || options.Session.Lease.Token == "" {
			return nil, fmt.Errorf("%s requires a valid session", operation)
		}
		headers.Set("X-RRD-Session", options.Session.Lease.SessionID)
		headers.Set("Authorization", "Bearer "+options.Session.Lease.Token)
	}
	var body []byte
	if descriptor.Method != http.MethodGet {
		resource := options.Resource
		if resource == nil {
			resource, err = defaultResource(c.instance, options.PathParameters)
		}
		if err != nil {
			return nil, err
		}
		resource, err = validateResource(resource)
		if err != nil {
			return nil, err
		}
		body, err = json.Marshal(map[string]any{
			"protocol":         "rrd",
			"protocol_version": 1,
			"context":          requestContext,
			"resource":         map[string]any{"segments": resource},
			"payload":          payload,
		})
		if err != nil {
			return nil, fmt.Errorf("encode RRD request: %w", err)
		}
		headers.Set("Content-Type", "application/json")
	}
	var lastErr error
	for attempt := 0; attempt < c.attemptLimit(descriptor, options); attempt++ {
		attemptContext, cancel, err := c.attemptContext(ctx, options.Deadline)
		if err != nil {
			return nil, err
		}
		request, err := http.NewRequestWithContext(
			attemptContext, descriptor.Method, target.String(), bytes.NewReader(body),
		)
		if err != nil {
			cancel()
			return nil, fmt.Errorf("construct RRD request: %w", err)
		}
		request.Header = headers.Clone()
		response, requestErr := c.http.Do(request)
		if requestErr != nil {
			cancel()
			lastErr = requestErr
			if ctx.Err() != nil {
				return nil, fmt.Errorf("RRD request cancelled: %w", ctx.Err())
			}
			continue
		}
		encoded, readErr := readBounded(response, c.maxResponseBytes)
		cancel()
		if readErr != nil {
			return nil, readErr
		}
		result, decodeErr := decodeResponse(response.StatusCode, encoded, requestContext)
		if decodeErr != nil {
			return nil, decodeErr
		}
		return result, nil
	}
	return nil, fmt.Errorf("RRD transport failed: %w", lastErr)
}

func makeContext(options RequestOptions, endpoint Endpoint) (*requestEnvelopeContext, error) {
	if endpoint.Method == http.MethodGet {
		return nil, nil
	}
	requestID, err := correlation(options.RequestID, "request ID")
	if err != nil {
		return nil, err
	}
	operationID, err := correlation(options.OperationID, "operation ID")
	if err != nil {
		return nil, err
	}
	var key string
	if options.IdempotencyKey != "" {
		key, err = correlation(options.IdempotencyKey, "idempotency key")
		if err != nil {
			return nil, err
		}
	}
	if endpoint.Mutation && key == "" {
		return nil, errors.New("mutating requests require an idempotency key")
	}
	deadline := int64(0)
	if !options.Deadline.IsZero() {
		deadline = options.Deadline.UnixMilli()
		if deadline <= 0 {
			return nil, errors.New("deadline must be a positive Unix millisecond instant")
		}
	}
	return &requestEnvelopeContext{
		RequestID: requestID, OperationID: operationID, IdempotencyKey: key, DeadlineUnixMS: deadline,
	}, nil
}

func defaultResource(instance string, parameters map[string]string) ([]ResourceSegment, error) {
	segments := make([]ResourceSegment, 0, 2)
	if engram := parameters["engram"]; engram != "" {
		identifier, err := canonical(engram, "engram")
		if err != nil {
			return nil, err
		}
		segments = append(segments, ResourceSegment{Kind: "engram", ID: identifier})
	}
	return append(segments, ResourceSegment{Kind: "instance", ID: instance}), nil
}

func validateResource(segments []ResourceSegment) ([]ResourceSegment, error) {
	if len(segments) < 1 || len(segments) > 16 {
		return nil, errors.New("resource paths must contain 1..=16 segments")
	}
	seen := make(map[string]struct{}, len(segments))
	result := make([]ResourceSegment, 0, len(segments))
	for _, segment := range segments {
		if _, known := resourceKinds[segment.Kind]; !known {
			return nil, errors.New("resource kind is unknown")
		}
		if _, duplicate := seen[segment.Kind]; duplicate {
			return nil, errors.New("resource kind is repeated")
		}
		identifier, err := canonical(segment.ID, "resource")
		if err != nil {
			return nil, err
		}
		seen[segment.Kind] = struct{}{}
		result = append(result, ResourceSegment{Kind: segment.Kind, ID: identifier})
	}
	return result, nil
}

func resolvePath(template string, parameters map[string]string) (string, error) {
	var replacementErr error
	result := pathParameter.ReplaceAllStringFunc(template, func(match string) string {
		name := match[1 : len(match)-1]
		value := parameters[name]
		if value == "" {
			replacementErr = fmt.Errorf("missing path parameter %s", name)
			return match
		}
		identifier, err := correlation(value, name+" path parameter")
		if err != nil {
			replacementErr = err
			return match
		}
		return url.PathEscape(identifier)
	})
	return result, replacementErr
}

func correlation(value, label string) (string, error) {
	if len(value) < 1 || len(value) > 128 || !correlationPattern.MatchString(value) {
		return "", fmt.Errorf("%s is not a canonical RRD correlation ID", label)
	}
	return value, nil
}

func canonical(value, label string) (string, error) {
	if len(value) < 1 || len(value) > 128 || !canonicalPattern.MatchString(value) {
		return "", fmt.Errorf("%s is not a canonical RRD identifier", label)
	}
	return value, nil
}
