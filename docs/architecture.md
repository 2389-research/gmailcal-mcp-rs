# Architecture

## Overview
The Gmail Calendar MCP Server provides an interface between Model Context Protocol (MCP) requests and Google's Gmail and Calendar APIs. It handles authentication, request processing, and response formatting.

## Components

### Main Server
Handles incoming MCP requests and routes them to appropriate handlers.

### Authentication Module
Manages OAuth2 flow with Google APIs.

### API Clients
- Gmail Client: Interacts with Gmail API
- Calendar Client: Interacts with Calendar API
- People Client: Interacts with People API

### Error Handling
Centralized error handling through the `errors` module.

## Request Flow
1. MCP client sends request to server
2. Server validates request format
3. Authentication tokens are verified
4. Request is routed to appropriate API client
5. Response is formatted and returned
