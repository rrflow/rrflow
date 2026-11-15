using System;
using System.Collections.Generic;
using System.Linq;
using System.Net.Http;

namespace Rrflow.Rrd;

internal sealed class OperationBinding
{
    private static readonly HashSet<string> ResourceKinds = new(StringComparer.Ordinal)
    {
        "organization", "engram", "project", "instance", "node", "shard", "collection",
        "table", "record", "transaction", "snapshot", "backup", "operation",
    };

    private OperationBinding(
        Endpoint endpoint,
        Uri target,
        Dictionary<string, object?>? requestContext,
        IReadOnlyList<ResourceSegment> resource,
        RequestOptions options)
    {
        Endpoint = endpoint;
        Target = target;
        RequestContext = requestContext;
        Resource = resource;
        Options = options;
    }

    internal Endpoint Endpoint { get; }
    internal Uri Target { get; }
    internal Dictionary<string, object?>? RequestContext { get; }
    internal IReadOnlyList<ResourceSegment> Resource { get; }
    private RequestOptions Options { get; }

    internal static OperationBinding Create(
        OperationId operation,
        RequestOptions options,
        EndpointResolver resolver)
    {
        Endpoint endpoint = EndpointCatalog.Get(operation);
        Dictionary<string, object?>? requestContext = MakeContext(endpoint, options);
        Uri target = resolver.Resolve(endpoint, options.PathParameters);
        IReadOnlyList<ResourceSegment> resource = endpoint.Method == "GET"
            ? Array.Empty<ResourceSegment>()
            : ValidateResource(
                options.Resource ?? DefaultResource(options.PathParameters, resolver.Instance));
        return new OperationBinding(endpoint, target, requestContext, resource, options);
    }

    internal void Authenticate(HttpRequestMessage request)
    {
        if (Endpoint.Authentication == Authentication.ApiKey)
        {
            if (Options.ApiKey is null || string.IsNullOrEmpty(Options.ApiKey.Credential))
            {
                throw new RrdClientException($"{Endpoint.WireName} requires API-key authentication");
            }
            request.Headers.Add(
                "X-RRD-Principal",
                EndpointResolver.CanonicalIdentifier(Options.ApiKey.PrincipalId, "principal"));
            request.Headers.TryAddWithoutValidation(
                "Authorization",
                "ApiKey " + Options.ApiKey.Credential);
        }
        else if (Endpoint.Authentication == Authentication.SessionBearer)
        {
            if (Options.Session is null
                || Options.Session.Lease.SessionId.Length == 0
                || Options.Session.Lease.Token.Length == 0)
            {
                throw new RrdClientException($"{Endpoint.WireName} requires a valid session");
            }
            request.Headers.Add("X-RRD-Session", Options.Session.Lease.SessionId);
            request.Headers.TryAddWithoutValidation(
                "Authorization",
                "Bearer " + Options.Session.Lease.Token);
        }
    }

    private static Dictionary<string, object?>? MakeContext(
        Endpoint endpoint,
        RequestOptions options)
    {
        if (endpoint.Method == "GET")
        {
            return null;
        }
        Dictionary<string, object?> context = new()
        {
            ["request_id"] = EndpointResolver.CorrelationIdentifier(
                options.RequestId,
                "request ID"),
            ["operation_id"] = EndpointResolver.CorrelationIdentifier(
                options.OperationId,
                "operation ID"),
        };
        if (options.IdempotencyKey is not null)
        {
            context["idempotency_key"] = EndpointResolver.CorrelationIdentifier(
                options.IdempotencyKey,
                "idempotency key");
        }
        else if (endpoint.Mutation)
        {
            throw new RrdClientException("mutating requests require an idempotency key");
        }
        if (options.Deadline is not null)
        {
            long milliseconds = options.Deadline.Value.ToUnixTimeMilliseconds();
            if (milliseconds <= 0)
            {
                throw new RrdClientException("deadline must be a positive Unix millisecond instant");
            }
            context["deadline_unix_ms"] = milliseconds;
        }
        return context;
    }

    private static IReadOnlyList<ResourceSegment> DefaultResource(
        IReadOnlyDictionary<string, string> parameters,
        string instance)
    {
        List<ResourceSegment> result = new();
        if (parameters.TryGetValue("engram", out string? engram))
        {
            result.Add(new ResourceSegment(
                "engram",
                EndpointResolver.CanonicalIdentifier(engram, "engram")));
        }
        result.Add(new ResourceSegment("instance", instance));
        return result;
    }

    private static IReadOnlyList<ResourceSegment> ValidateResource(
        IReadOnlyList<ResourceSegment> segments)
    {
        if (segments.Count is < 1 or > 16)
        {
            throw new RrdClientException("resource paths must contain 1..=16 segments");
        }
        HashSet<string> kinds = new(StringComparer.Ordinal);
        return segments.Select(segment =>
        {
            if (!ResourceKinds.Contains(segment.Kind) || !kinds.Add(segment.Kind))
            {
                throw new RrdClientException("resource kind is unknown or repeated");
            }
            return new ResourceSegment(
                segment.Kind,
                EndpointResolver.CanonicalIdentifier(segment.Id, "resource"));
        }).ToArray();
    }
}
