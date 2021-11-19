export interface ViewerStyleConfig {
    propertyName: string,
    valueRange: number[],
    colorRange: string[],
}
export interface ViewerConfig {
    baseUrl: string,
    datasetName: string,
    h3indexPropertyName: string,
    styleConfig: ViewerStyleConfig | undefined | null
}

export function getViewerConfig()  {
    // @ts-ignore
    return document.viewer_config as ViewerConfig;
}
