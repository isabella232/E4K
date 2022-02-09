# IoTEdge SPIFFE Server

The IoTEdge SPIFFE Server is responsible for: 
-	Managing the key to sign the SVIDs
-	Signing the SVIDs
-	Managing the trust bundle
-	Attest the agents.

_Figure : IoTEdge SPIFFE Server_

<img src="drawings/IoTEdge_SPIFFE_server.svg"/>

“Admin inputs” are the configuration parameters from the “Identity Manager”. “Admin inputs” are a mix of:
-	 The configuration parameters like which plugin to use, configuration for the server API and more.
-	The SVIDs entries generated from the identity manager configuration.

The IoTEdge SPIFFE Server waits for the IoTEdge SPIFFE agent to connect. When an IoTEdge SPIFFE Agent connects for the first time, the server performs a node attestation together with the agent. 

After attestation, the IoTEdge SPIFFE server delivers a JWTSVID to the agent for future communications. Once the IoTEdge SPIFFE Agent receives its JWTSVID, it can communicate with the IoTEdge SPIFFE Server to get the trust bundle and the SVIDs.
-	The SVIDs are crafted based on Generating JSON Web Token structure using the signing key stored by the key plugin. 
-	The Trust bundle is also recorded in the common database since the trust bundle is a merge of the public keys of all the IoTEdge SPIFFE Server replicas. When there is a change in the Trust Bundle, the IoTEdge SPIFFE Agents are automatically notified.

The background task represents background operations like regularly rotating the signing keys.

# Admin APIs
---
## Get entries
Get all entries. Because of possible flood of entried, results are paginated.
### Request
```
GET   /entries?api-version=2022_06_01&page_size={uint32}&page_token={string}
```

#### Params
```
page_size : uint32: The maximum number of results to return.
page_token: optional string: The page token
```
### Response
```
200 OK

content-type: application/json
```
### Response Body
```
{
    "entries" : [ 
        { 
          "id" : "string: Hash of the entry. Important if product is scaled horizontally. Replicas need to generate the same key",
          "iot_hub_id" : { (Optional)
            "iot_hub_hostname" : "string: IoTHub hostname",
            "device_id" : "string: device id",
            "module_id" : "string: module id"
          }
          "spiffe_id" : "string: The SPIFFE ID of the identity described by this entry."
          "parent_id" : "optional string: who the entry is delegated to. If none, node selector must be used."
          "selectors" : ["string: selector1", "string: selector2", "...],
          "ttl" : "uint64, svid time to live",
          "admin" : "bool: Admin workload",
          "expires_at" : "uint64: seconds since Unix epoch, when the entry expires",
          "dns_names" : ["string: used for crafting certificate"],
          "revision_number" : "uint64: version number of the entrie, bump when updated",
          "store_svid" : "bool: Determines if the issued identity is exportable to a store"
        },
        ...
    ],
    "page_token" "optional string: The page token. None if no more pages"    
}
```
---
## Create entries
Create entries that are entitled to SVIDs in IoTEdge SPIFFE Server. 
Gives access to related workload to the workload API.
### Request
```
POST   /entries?api-version=2022_06_01
```
#### Request Body
```
{
    "entries" : [ 
        { 
          "id" : "string: Hash of the entry. Important if product is scaled horizontally. Replicas need to generate the same key",
          "iot_hub_id" : { (Optional)
            "iot_hub_hostname" : "string: IoTHub hostname",
            "device_id" : "string: device id",
            "module_id" : "string: module id"
          }
          "spiffe_id" : "string: The SPIFFE ID of the identity described by this entry."
          "parent_id" : "optional string: who the entry is delegated to. If none, node selector must be used."
          "selectors" : ["string: selector1", "string: selector2", "...],
          "ttl" : "uint64, svid time to live",
          "admin" : "bool: Admin workload",
          "expires_at" : "uint64: seconds since Unix epoch, when the entry expires",
          "dns_names" : ["string: used for crafting certificate"],
          "revision_number" : "uint64: version number of the entrie, bump when updated",
          "store_svid" : "bool: Determines if the issued identity is exportable to a store"
        },
        ...
    ]
}
```
### Response
```
201 CREATED

content-type: application/json
```
### Response Body
```
{
    "results" : [ 
        { 
          "id" : "string: Hash of the entry. Important if product is scaled horizontally. Replicas need to generate the same key",
          "status" : "Error Status"
        },
        ...
    ]
}
```
## Update entries
Update entries in the IoTEdge SPIFFE Server
### Request
```
PUT   /entries?api-version=2022_06_01
```
#### Request Body
```
{
    "entries" : [ 
        { 
          "id" : "string: Hash of the entry. Important if product is scaled horizontally. Replicas need to generate the same key",
          "iot_hub_id" : { (Optional)
            "iot_hub_hostname" : "string: IoTHub hostname",
            "device_id" : "string: device id",
            "module_id" : "string: module id"
          }
          "spiffe_id" : "string: The SPIFFE ID of the identity described by this entry."
          "parent_id" : "optional string: who the entry is delegated to. If none, node selector must be used."
          "selectors" : ["string: selector1", "string: selector2", "...],
          "ttl" : "uint64, svid time to live",
          "admin" : "bool: Admin workload",
          "expires_at" : "uint64: seconds since Unix epoch, when the entry expires",
          "dns_names" : ["string: used for crafting certificate"],
          "revision_number" : "uint64: version number of the entrie, bump when updated",
          "store_svid" : "bool: Determines if the issued identity is exportable to a store"
        },
        ...
    ]
}
```
### Response
```
200 OK

content-type: application/json
```
### Response Body
```
{
    "results" : [ 
        { 
          "id" : "string: Hash of the entry. Important if product is scaled horizontally. Replicas need to generate the same key",
          "status" : "Error Status"
        },
        ...
    ]
}
```

---
## Delete entries
Delete entries in the IoTEdge SPIFFE Server. Deleting an entry will revoke access of the related workload to the workload API.
### Request
```
DEL   /entries?api-version=2022_06_01
```
#### Request Body
```
{
    "ids" : ["string: id1", "string: id2", ...]
}
```
### Response
```
200 OK

content-type: application/json
```
### Response Body
```
{
    "results" : [ 
        { 
          "id" : "string: Hash of the entry. Important if product is scaled horizontally. Replicas need to generate the same key",
          "status" : "Error Status"
        },
        ...
    ]
}
```

---
## Get entries 
Get the entries specified in the request.
### Request
```
POST   /select-listEntries?api-version=2022_06_01
```

#### Request Body
```
{
    "ids" : ["string: id1", "string: id2", ...]
}
```
### Response
```
200 OK

content-type: application/json
```
### Response Body
```
{
    "entries" : [ 
        { 
          "id" : "string: Hash of the entry. Important if product is scaled horizontally. Replicas need to generate the same key",
          "iot_hub_id" : { (Optional)
            "iot_hub_hostname" : "string: IoTHub hostname",
            "device_id" : "string: device id",
            "module_id" : "string: module id"
          }
          "spiffe_id" : "string: The SPIFFE ID of the identity described by this entry."
          "parent_id" : "optional string: who the entry is delegated to. If none, node selector must be used."
          "selectors" : ["string: selector1", "string: selector2", "...],
          "ttl" : "uint64, svid time to live",
          "admin" : "bool: Admin workload",
          "expires_at" : "uint64: seconds since Unix epoch, when the entry expires",
          "dns_names" : ["string: used for crafting certificate"],
          "revision_number" : "uint64: version number of the entrie, bump when updated",
          "store_svid" : "bool: Determines if the issued identity is exportable to a store"
        },
        ...
    ]
}
```
---
## Configure IoTEdge SPIRE Server
Configure SPIRE server. Configuring again will remove existing configuration.
### Request
```
POST   /configuration?api-version=2022_06_01
```
#### Request Body
```
{
    "trust_domain" : "string: SPIFFE ID trust domain",
    "Node_attestor_plugin" : "string: How node are attested",
    "Workload_attestor_plugin" : "string: How workload are attested" 
}
```
### Response

```
201 Created

content-type: application/json
```
---
# Server APIs
---
## Create and Get new JWTSVID
Request the server to create a new JWTSVID, sign it and return it

### Request
```
POST   /new-JWT-SVID?api-version=2022_06_01
```
#### Request Body
```
{ 
  "id" : "string: Hash of the entry. Important if product is scaled horizontally. Replicas need to generate the same key",
  "audience" : "string: list of audience for the JWT. At least one audience is required."
}
```
### Response
```
201 CREATED

content-type: application/json
```
### Response Body
```
{
    "jwt_svid" : {
        "token" : "string: Compact representation of the JWTSVID",
        "spiffe_id" : {
            "trust_domain" : "string: The trust domain",
            "path" : "string: The path component of the SPIFFE ID"
        },
        "expires_at" : "uint64: Expiration timestamp (seconds since Unix epoch).",
        "issued_at" : "uint64: Issuance timestamp (seconds since Unix epoch)."     
    }
}
```
---
## Get Trust Bundle
Gets the bundle for the trust domain of the server.

### Request
```
GET   /trust-bundle?api-version=2022_06_01&jwt_keys={bool}&x509_cas={bool}
```
#### Params
```
jwt_keys : bool: If true jwt_keys are included"
x509_cas: "bool: If true x509_cas are included"
```
### Response
```
200 OK

content-type: application/json
```
### Response Body
```
{
    "bundle" : {
        "trust_domain" : "string: The trust domain",
        "jwt_keys" : [{ (Optional, keys to authenticate the JWT => JWK)
            "public_key" : "byte: The PKIX encoded public key.",
            "key_id" : "string: The key identifier.",
            "expires_at" : "uint64: Expiry time in seconds since Unix epoch",
        },
        ...
        ],
        "x509_cas" : [{(Optional)
            "bytes" : "bytes : The ASN.1 DER encoded bytes of the X.509 certificate"
        },
        ...
        ],
        "refresh_hint" : "uint64: How often the trust bundle should be refreshed, in second",
        "sequence_number" : "uint64: The sequence number of the bundle." 
    }
}
```
