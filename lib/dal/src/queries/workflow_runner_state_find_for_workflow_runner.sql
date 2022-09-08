SELECT DISTINCT ON (id) id,
                        visibility_change_set_pk,
                        visibility_deleted_at,
                        row_to_json(workflow_runner_states.*) AS object

FROM workflow_runner_states
WHERE in_tenancy_v1($1, tenancy_universal, tenancy_billing_account_ids,
                    tenancy_organization_ids, tenancy_workspace_ids)
  AND is_visible_v1($2, visibility_change_set_pk,
                    visibility_deleted_at)
  AND workflow_runner_id = $3

ORDER BY id,
         visibility_change_set_pk DESC,
         visibility_deleted_at DESC NULLS FIRST;
