#!/usr/bin/env node
import * as cdk from 'aws-cdk-lib';
import { InfrastructureStack } from '../lib/infrastructure-stack';

const projectName = 'smallworld';

const {
	swm_account: account,
	swm_region: region,
	swm_certificate_id: certificateId,
	swm_domain_name: domainName,
} = process.env;

const stage = process.env.swm_stage ?? 'prod';

if (stage !== 'prod') {
	throw new Error(`Invalid stage: ${stage}`);
}
if (!account || !certificateId || !domainName || !region) {
	throw new Error(`Please provide all of the following configuration (in the environment):

swm_account          The AWS account identifier.
swm_region           The AWS region.
swm_stage            The deployment stage (ex, prod).
swm_domain_name      A domain name for which there exists a hosted zone in AWS.
swm_certificate_id   An AWS ACM certificate id that is valid for the provided
                     domain name, and the sub-domain with name "smallworld".
					 This certificate must be created in the us-east-1 region,
					 even if the deployment region is something else.

`);
}

const app = new cdk.App();

const stack = new InfrastructureStack(app, `${projectName}-${stage}-stack`, {
	env: { account, region },
	projectName,
	stage,
	certificateId,
	domainName,
});

stack.setup();
