@Library('jenkins-library@feature/DOPS-2261/iroha2-pr-deploy') _

def pipeline = new org.iroha2.AppPipeline(steps: this,
    k8sPrDeploy: true,
    vaultPrPath: "argocd-cc/src/charts/iroha2/environments/tachi/",
    vaultUser: "iroha2-rw",
    vaultCredId: "iroha2VaultCreds",
    valuesDestPath: "argocd-cc/src/charts/iroha2/",
    devValuesPath: "dev/dev/",
    initialSecretName: "iroha2-eso-base",
    initialNameSpace: "iroha2-dev",
    targetNameSpace: "iroha2-${env.CHANGE_ID}-dev-web",
    targetSecretName: "iroha2-${env.CHANGE_ID}-dev-web-eso-base",
    ingressEnabled: false
)
pipeline.runPipeline()