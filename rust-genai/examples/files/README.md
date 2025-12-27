# Examples Files

该目录用于管理示例的输入与输出文件：

- input/：示例使用的固定输入文件（会随 crate 打包发布）。
- output/：运行示例时生成的输出文件（仅运行时生成，不随 crate 打包）。

默认输出目录为 `examples/files/output`，可通过环境变量 `GENAI_EXAMPLE_FILES_DIR` 覆盖。
