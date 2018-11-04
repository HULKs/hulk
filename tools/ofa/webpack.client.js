module.exports = {
    entry: "./src/ofa/client/index.tsx",
    output: {
        filename: "index.js",
        path: __dirname + "/build/ofa/public"
    },
    devtool: "source-map",
    resolve: {
        extensions: [".ts", ".tsx", ".js", ".json"]
    },
    module: {
        rules: [
            {
                test: /\.tsx?$/,
                loader: "awesome-typescript-loader",
                options: {
                    configFileName: __dirname + "/tsconfig.client.json"
                }
            },
            {
                test: /\.js?$/,
                loader: "awesome-typescript-loader",
                options: {
                    configFileName: __dirname + "/tsconfig.client.json"
                }
            },
            {
                test: /\.css$/,
                use: ['style-loader', 'css-loader']
            },
            {
                test: /\.(woff|woff2|eot|ttf|otf)$/,
                use: ['file-loader']
            }
        ]
    }
};
