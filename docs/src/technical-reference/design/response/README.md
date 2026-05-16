# Response

## Workflow

```mermaid
flowchart TD

    A[request received]
    OP1(OPTIONS method ?)
    OP2[200 OK]
    OP3((end))
    B[logger dumps request]
    C(middlewares enabled?)
    MW[[middleware]]
    MWC("completed ?")
    MWCZ((end))
    D(rule sets enabled?)
    RR[[rule-based routing]]
    RRC("completed ?")
    RRCZ((end))
    FR[[file-based routing]]
    FRC("completed ?")
    FRCZ((end))
    Z2[not found response]
    Z2Z((end))

    A --> OP1
    OP1 -->|Yes| OP2
    OP1 -->|No| B
    OP2 --> OP3
    B --> C
    C -->|Yes| MW
    C -->|No| D
    MW --> MWC
    MWC -->|Yes| MWCZ
    MWC -->|No| D
    D --> |Yes| RR
    D --> |No| FR
    RR --> RRC
    RRC -->|Yes| RRCZ
    RRC -->|No| FR
    FR --> FRC
    FRC -->|Yes| FRCZ
    FRC -->|No| Z2
    Z2 --> Z2Z
```

## Middleware

```mermaid
flowchart TD

    subgraph MW[for each middleware]
        MW1[run rhai]
        MW2(response resource returned ?)

        subgraph MWR[return response]
            MWR1(resource type ?)
            MWR2(which map key found ?)
            MWR3(file content got ?)
            MWR4(valid json ?)
            MWR5[return response]
            MWR6[error response]
            MWRZ(("continue"))
        end

        Z(("continue"))
    end

    MW1 -->MW2
    MW2 -->|Yes| MWR
    MW2 -->|No| Z
    MWR1 -->|string| MWR3
    MWR1 -->|map| MWR2
    MWR2 -->|file_path| MWR3
    MWR2 -->|json| MWR4
    MWR2 -->|text| MWR5
    MWR2 -->|"(none)"| MWRZ
    MWR3 -->|Yes| MWR5
    MWR3 -->|No| MWR6
    MWR4 -->|Yes| MWR5
    MWR4 -->|No| MWR6
```

## Rule-based routing

```mermaid
flowchart TD

    subgraph RS[for each rule set]
      subgraph RR[for each rule]
          RR1["is_match()"]
          RR2(matched entry found ?)
          RR2Z((continue))

          subgraph RRR[return response]
              RRR1(file_path or text ?)
              RRR2(content got ?)
              RRR3[return response]
              RRR4[error response]
          end
      end
    end

    RR1 -->RR2
    RR2 -->|Yes| RRR
    RR2 -->|No| RR2Z
    RRR1 -->|file_path| RRR2
    RRR1 -->|text| RRR3
    RRR2 -->|Yes| RRR3
    RRR2 -->|No| RRR4
```

## File-based routing

- try to use request url_path as file path from fallback_respond_dir
- try to find file as it is and then ones which are attached supported extensions to 

```mermaid
flowchart TD

    subgraph FSE["for each raw path or extension-attached"]
        FSE1(file found ?)
        FSE1Z((continue))

        subgraph FSER[return response]
            FSER1(file_path or text ?)
            FSER2(content got ?)
            FSER3[return response]
            FSER4[error response]
        end
    end

    FSE1 -->|Yes| FSER
    FSE1 -->|No| FSE1Z
    FSER1 -->|file_path| FSER2
    FSER1 -->|text| FSER3
    FSER2 -->|Yes| FSER3
    FSER2 -->|No| FSER4
```