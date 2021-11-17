const path = require('path');

// https://webpack.js.org/guides/typescript/

const mode = process.env.NODE_ENV || 'development';

let devtool = false;
if (mode === 'development') {
    devtool = 'eval-source-map';
}

module.exports = {
    entry: './src/index.ts',
    mode: mode,
    module: {
        rules: [
            {
                test: /\.tsx?$/,
                use: 'ts-loader',
                exclude: /node_modules/,
            },
            {
                test: /\.css$/i,
                use: ["style-loader", "css-loader"],
            },
        ],
    },
    resolve: {
        extensions: ['.tsx', '.ts', '.js'],
    },
    output: {
        filename: 'viewer.js',
        path: path.resolve(__dirname, 'dist'),
    },
    devtool: devtool
};
