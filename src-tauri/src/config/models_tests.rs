use super::*;

#[test]
fn test_board_def_serde_stages_and_columns() {
    let json_both = r#"{"id":"dev","name":"Dev","stages":[{"id":"s1","title":"S1"}],"columns":[{"id":"s2","title":"S2"}]}"#;
    let board_both: BoardDef = serde_json::from_str(json_both).expect("deserialize both");
    assert_eq!(board_both.stages.len(), 1);
    assert_eq!(board_both.stages[0].id, "s1");

    let json_cols = r#"{"id":"dev","name":"Dev","columns":[{"id":"s2","title":"S2"}]}"#;
    let board_cols: BoardDef = serde_json::from_str(json_cols).expect("deserialize columns");
    assert_eq!(board_cols.stages.len(), 1);
    assert_eq!(board_cols.stages[0].id, "s2");

    let json_stages = r#"{"id":"dev","name":"Dev","stages":[{"id":"s3","title":"S3"}]}"#;
    let board_stages: BoardDef = serde_json::from_str(json_stages).expect("deserialize stages");
    assert_eq!(board_stages.stages.len(), 1);
    assert_eq!(board_stages.stages[0].id, "s3");
}

#[test]
fn test_pipeline_config_serde_stages_and_columns() {
    let json_both = r#"{"id":"p1","title":"P1","stages":[{"id":"s1","title":"S1"}],"columns":[{"id":"s2","title":"S2"}]}"#;
    let pipe_both: PipelineConfig = serde_json::from_str(json_both).expect("deserialize pipe both");
    assert_eq!(pipe_both.stages.len(), 1);
    assert_eq!(pipe_both.stages[0].id, "s1");

    let json_cols = r#"{"id":"p1","title":"P1","columns":[{"id":"s2","title":"S2"}]}"#;
    let pipe_cols: PipelineConfig = serde_json::from_str(json_cols).expect("deserialize pipe columns");
    assert_eq!(pipe_cols.stages.len(), 1);
    assert_eq!(pipe_cols.stages[0].id, "s2");
}

#[test]
fn test_pipeline_stage_wip_limit_serde() {
    let json_camel = r#"{"id":"dev","title":"Development","wipLimit":4}"#;
    let stage: PipelineStage = serde_json::from_str(json_camel).expect("deserialize camelCase wipLimit");
    assert_eq!(stage.wip_limit, Some(4));

    let json_snake = r#"{"id":"dev","title":"Development","wip_limit":7}"#;
    let stage_snake: PipelineStage = serde_json::from_str(json_snake).expect("deserialize snake_case wip_limit");
    assert_eq!(stage_snake.wip_limit, Some(7));

    let serialized = serde_json::to_string(&stage).expect("serialize");
    assert!(serialized.contains("\"wipLimit\":4"));

    let stage_none: PipelineStage = serde_json::from_str(r#"{"id":"dev","title":"Development"}"#).expect("deserialize none");
    assert_eq!(stage_none.wip_limit, None);
}
